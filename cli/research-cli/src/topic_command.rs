//! The `research topic` command layer: a set's round plan rendered as the
//! JSON `research-topic`'s `conduct` consumes, and the run ledger `conduct`
//! keeps between plans.

use std::fmt::Write as _;
use std::num::NonZeroU32;
use std::path::Path;
use std::path::PathBuf;

use corpus::scan::DirReader;
use corpus::scan::DirectoryProbe;
use corpus::scan::FileReader;
use corpus::AtomicWrite;
use corpus::DocTypeKey;
use corpus::FileRemove;
use corpus_adapters::resolve::resolve_document;
use corpus_adapters::resolve::Resolution;
use corpus_adapters::YamlFrontmatter;
use research::conduct::concurrency::Provenance;
use research::conduct::concurrency::RequestedConcurrency;
use research::conduct::concurrency::ResolvedConcurrency;
use research::conduct::ledger::RunId;
use research::conduct::ledger::RunLedger;
use research::conduct::memory::Withheld;
use research::conduct::observed::Observed;
use research::conduct::replaced_run::Leftover;
use research::conduct::replaced_run::Recovery;
use research::conduct::replaced_run::ReplacedRun;
use research::conduct::search_budget::unsearched_tree;
use research::conduct::search_budget::SearchBudgets;
use research::conduct::search_budget::SEARCHING_PROFILE;
use research::conduct::spawn::SpawnRef;
use research::conduct::window::window;
use research::conduct::window::UnfinishedSpawn;
use research::conduct::window::Window;
use research::topic::layout::lineage::Lineage;
use research::topic::layout::note_ref::NoteRef;
use research::topic::plan::OutstandingPair;
use research::topic::plan::RoundInputs;
use research::topic::plan::RoundPlan;
use research::topic::plan::ShallowFinding;
use research::topic::tree::Depth;
use research::topic::tree::MissingNode;
use research::topic::tree::Stage;
use research::topic::tree::Trim;
use research_adapters::conduct::ledger::delete_run_ledger;
use research_adapters::conduct::ledger::read_run_ledger;
use research_adapters::conduct::ledger::run_ledger_path;
use research_adapters::conduct::ledger::write_run_ledger;
use research_adapters::conduct::ledger::LedgerError;
use research_adapters::topic::read_round_inputs;
use research_adapters::unicode_text::UnicodeTables;
use serde_json::json;
use serde_json::Value;

use crate::concurrency_wording;
use crate::context::ProjectContext;
use crate::ledger_wording;
use crate::plan_wording;
use crate::search_cap::EnvironmentSearchCap;
use crate::search_wording;
use crate::subagent_cap::EnvironmentCap;

const DOC_TYPE: &str = "topic-research";

/// What one call prints: the plan on stdout, its warnings on stderr.
#[derive(Debug, Default)]
pub struct Printed {
    pub stdout: String,
    pub stderr: String,
}

/// The canonical root of the set `slug` names.
///
/// # Errors
///
/// A message when the configured directory cannot be read, or when `slug`
/// resolves to no set.
pub fn resolve_set(
    project: &ProjectContext,
    cwd: &Path,
    slug: &str,
) -> Result<PathBuf, String> {
    let sets = project.type_dir(DocTypeKey::TopicResearch)?;
    let Resolution::Resolved(set_root) =
        resolve_document(cwd, DOC_TYPE, slug, |_| Ok(sets))
    else {
        return Err(format!(
            "E_TOPIC_RESEARCH_UNRESOLVED: no topic-research set '{slug}'"
        ));
    };
    Ok(set_root)
}

/// Whether a plan belongs to a `conduct` run, and which.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunMode {
    Query,
    Start(RunId),
    Continue {
        run: RunId,
        spawned: Option<u32>,
        withheld: Withheld,
    },
}

/// The run flags as given, before a started run is minted its id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunFlag {
    Query,
    Start,
    Continue {
        run: RunId,
        spawned: Option<u32>,
        withheld: Withheld,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutstandingFlags {
    pub depth: Depth,
    pub requested_concurrency: Option<RequestedConcurrency>,
    pub run: RunFlag,
}

impl OutstandingFlags {
    /// # Errors
    ///
    /// A message naming the first flag that is malformed or
    /// combined with one it excludes.
    pub fn parse(
        depth: &str,
        concurrency: Option<&str>,
        start: bool,
        run: Option<&str>,
        spawned: Option<&str>,
        withhold_profile: &[String],
    ) -> Result<Self, String> {
        let depth = non_negative(depth).and_then(Depth::requested).ok_or_else(
            || {
                failed(format!(
                    "E_TOPIC_RESEARCH_DEPTH: --depth must be 1 to {}, got \
                     '{depth}'",
                    Depth::DEEPEST
                ))
            },
        )?;
        Ok(Self {
            depth,
            requested_concurrency: concurrency.map(|value| {
                RequestedConcurrency::Given {
                    value: value.to_owned(),
                    provenance: Provenance::Flag,
                }
            }),
            run: run_flag(start, run, spawned, withhold_profile)?,
        })
    }
}

pub struct PlanRequest {
    pub depth: Depth,
    pub concurrency: CappedConcurrency,
    pub search_cap: EnvironmentSearchCap,
    pub mode: RunMode,
}

pub struct CappedConcurrency {
    pub resolved: ResolvedConcurrency,
    pub cap: EnvironmentCap,
}

/// # Errors
///
/// A message when the configured value cannot be read, or when the
/// requested concurrency is empty.
pub fn resolve_concurrency(
    flag: Option<RequestedConcurrency>,
    configured: impl FnOnce() -> Result<RequestedConcurrency, String>,
    cap: EnvironmentCap,
) -> Result<CappedConcurrency, String> {
    let requested = match flag {
        Some(requested) => requested,
        None => configured()?,
    };
    requested
        .resolve(cap.cap)
        .map(|resolved| CappedConcurrency { resolved, cap })
        .map_err(|empty| {
            failed(format!(
                "E_TOPIC_RESEARCH_CONCURRENCY: {}",
                concurrency_wording::empty_concurrency(empty)
            ))
        })
}

fn run_flag(
    start: bool,
    run: Option<&str>,
    spawned: Option<&str>,
    withhold_profile: &[String],
) -> Result<RunFlag, String> {
    if start && run.is_some() {
        return Err(failed(
            "E_TOPIC_RESEARCH_RUN: --start and --run cannot be combined",
        ));
    }
    let Some(run) = run else {
        if spawned.is_some() {
            return Err(failed(
                "E_TOPIC_RESEARCH_SPAWNED: --spawned needs --run",
            ));
        }
        if !withhold_profile.is_empty() {
            return Err(failed(
                "E_TOPIC_RESEARCH_WITHHOLD_PROFILE: --withhold-profile needs \
                 --run",
            ));
        }
        return Ok(if start {
            RunFlag::Start
        } else {
            RunFlag::Query
        });
    };
    let spawned = spawned
        .map(|spawned| {
            non_negative(spawned).ok_or_else(|| {
                failed(format!(
                    "E_TOPIC_RESEARCH_SPAWNED: --spawned must be a \
                     non-negative integer, got '{spawned}'"
                ))
            })
        })
        .transpose()?;
    Ok(RunFlag::Continue {
        run: parse_run(run)?,
        spawned,
        withheld: withheld(withhold_profile)?,
    })
}

fn withheld(withhold_profile: &[String]) -> Result<Withheld, String> {
    if withhold_profile.iter().any(String::is_empty) {
        return Err(failed(
            "E_TOPIC_RESEARCH_WITHHOLD_PROFILE: --withhold-profile is empty; \
             pass a profile name",
        ));
    }
    Ok(Withheld::of(withhold_profile.iter().cloned()))
}

/// # Errors
///
/// A message when `text` is not a run id.
pub fn parse_run(text: &str) -> Result<RunId, String> {
    RunId::parse(text).ok_or_else(|| {
        failed(format!(
            "E_TOPIC_RESEARCH_RUN: --run must be 1 to 64 letters, digits, \
             '_' or '-', got '{text}'"
        ))
    })
}

fn non_negative(text: &str) -> Option<u32> {
    let digits = !text.is_empty() && text.bytes().all(|b| b.is_ascii_digit());
    digits.then(|| text.parse().ok()).flatten()
}

fn failed(message: impl Into<String>) -> String {
    message.into()
}

/// Runs `research topic outstanding` against the set rooted at `set_root`,
/// which must be canonical: every path is emitted beneath it.
///
/// A run's ledger is written before the plan is rendered, so `conduct` never
/// spawns a batch the ledger has not recorded.
///
/// # Errors
///
/// A message when the set, the profiles directory or the run's
/// ledger cannot be read, when the run was superseded, or when the ledger
/// cannot be written.
pub fn run_outstanding<F, S>(
    set_root: &Path,
    profiles_dir: &Path,
    request: &PlanRequest,
    fs: &F,
    store: &S,
) -> Result<Printed, String>
where
    F: DirReader + FileReader + DirectoryProbe,
    S: AtomicWrite,
{
    let mut notices = Vec::new();
    planned(set_root, profiles_dir, request, fs, store, &mut notices).map_err(
        |error| format!("{}{error}", warning_lines(notices.iter().cloned())),
    )
}

fn planned<F, S>(
    set_root: &Path,
    profiles_dir: &Path,
    request: &PlanRequest,
    fs: &F,
    store: &S,
    notices: &mut Vec<String>,
) -> Result<Printed, String>
where
    F: DirReader + FileReader + DirectoryProbe,
    S: AtomicWrite,
{
    let (continued, replaced) = match &request.mode {
        RunMode::Continue {
            run,
            spawned,
            withheld,
        } => (
            Some(
                continued_ledger(set_root, run, *spawned, fs)?
                    .withholding(withheld),
            ),
            ReplacedLedger::Absent,
        ),
        RunMode::Start(_) => {
            notices.extend(start_notices(&request.concurrency));
            let replaced = replaced_ledger(set_root, fs)?;
            if matches!(replaced, ReplacedLedger::Corrupt) {
                notices.push(ledger_wording::corrupt_notice());
            }
            (None, replaced)
        }
        RunMode::Query => (None, ReplacedLedger::Absent),
    };
    let claims = continued
        .as_ref()
        .map(|ledger| ledger.claims().clone())
        .unwrap_or_default();
    let reading = read_round_inputs(
        set_root,
        profiles_dir,
        request.depth,
        claims,
        fs,
        &YamlFrontmatter,
        &UnicodeTables,
    )
    .map_err(|error| error.to_string())?;
    let continued = continued
        .map(|ledger| settled(ledger, &reading.inputs, set_root))
        .transpose()?;
    let (inputs, plan, replaced_run, owed_quarantine) = match &replaced {
        ReplacedLedger::Readable(stored) => {
            let recovery = Recovery::of(stored, reading.inputs, &UnicodeTables);
            (
                recovery.inputs,
                recovery.plan,
                Some(recovery.replaced_run),
                recovery.owed_quarantine,
            )
        }
        ReplacedLedger::Absent | ReplacedLedger::Corrupt => {
            let plan = RoundPlan::of(&reading.inputs, &UnicodeTables);
            (reading.inputs, plan, None, Vec::new())
        }
    };
    if matches!(request.mode, RunMode::Start(_)) {
        notices
            .extend(unhonoured_search_cap_notice(&request.search_cap, &plan));
    }
    let observed = Observed::of(&inputs);
    let ledger = match &request.mode {
        RunMode::Start(run) => Some(
            RunLedger::start(run.clone(), &observed)
                .owing_quarantine(&owed_quarantine),
        ),
        RunMode::Continue { .. } => continued,
        RunMode::Query => None,
    };
    let ledger = ledger
        .map(|ledger| budgeted(ledger, &plan, &request.search_cap, notices));
    let window = window(
        &plan,
        &observed,
        request.concurrency.resolved.concurrency,
        ledger.as_ref().map(RunLedger::memory),
    );
    let ledger = ledger.map(|ledger| ledger.record(&plan, &observed, &window));
    if let Some(ledger) = &ledger {
        stored(set_root, ledger, store)?;
    }
    let stderr = printed_warnings(notices, replaced_run.as_ref(), &plan);
    let warnings = plan
        .warnings
        .iter()
        .map(plan_wording::warning)
        .chain(reading.warnings.iter().map(ToString::to_string))
        .chain(notices.iter().cloned())
        .collect();
    let rendered = PlanOutput {
        plan: &plan,
        window: &window,
        set_root,
        request,
        ledger: ledger.as_ref(),
        replaced_run: replaced_run.as_ref(),
        warnings,
    };
    Ok(Printed {
        stdout: format!("{}\n", rendered.render()),
        stderr,
    })
}

/// Runs `research topic end-run`: removes the set's ledger when `run` owns
/// it.
///
/// # Errors
///
/// A message when another run owns the ledger, or it cannot be
/// read or removed.
pub fn run_end_run<F: FileReader, R: FileRemove>(
    set_root: &Path,
    run: &RunId,
    fs: &F,
    remover: &R,
) -> Result<Printed, String> {
    match read_run_ledger(set_root, fs, &UnicodeTables) {
        Ok(None) => Ok(Printed::default()),
        Ok(Some(ledger)) if ledger.run() == run => {
            delete_run_ledger(set_root, remover).map_err(|error| {
                failed(format!(
                    "E_TOPIC_RESEARCH_RUN_LEDGER: could not remove {}: \
                     {error}",
                    run_ledger_path(set_root).display()
                ))
            })?;
            Ok(Printed::default())
        }
        Ok(Some(ledger)) => Err(superseded(run, ledger.run())),
        Err(LedgerError::Corrupt) => Err(no_usable_ledger(run)),
        Err(LedgerError::Unreadable(error)) => Err(error.to_string()),
    }
}

fn continued_ledger<F: FileReader>(
    set_root: &Path,
    run: &RunId,
    spawned: Option<u32>,
    fs: &F,
) -> Result<RunLedger, String> {
    match read_run_ledger(set_root, fs, &UnicodeTables) {
        Ok(Some(stored)) => RunLedger::continue_as(stored, run, spawned)
            .map_err(|owner| superseded(run, &owner.by)),
        Ok(None) | Err(LedgerError::Corrupt) => Err(no_usable_ledger(run)),
        Err(LedgerError::Unreadable(error)) => Err(error.to_string()),
    }
}

fn settled(
    ledger: RunLedger,
    inputs: &RoundInputs,
    set_root: &Path,
) -> Result<RunLedger, String> {
    let run = ledger.run().clone();
    ledger.settling_quarantine(inputs).map_err(|owed| {
        let paths: Vec<String> = owed
            .stems
            .iter()
            .map(|stem| {
                set_root
                    .join(stem.finding_path())
                    .to_string_lossy()
                    .into_owned()
            })
            .collect();
        failed(format!(
            "E_TOPIC_RESEARCH_QUARANTINE_OWED: run {run} still owes the \
             quarantine of {}; rename each to .<name>.invalid beside it, \
             never deleting it, then re-run conduct",
            paths.join(", ")
        ))
    })
}

fn stored<S: AtomicWrite>(
    set_root: &Path,
    ledger: &RunLedger,
    store: &S,
) -> Result<(), String> {
    write_run_ledger(set_root, ledger, store).map_err(|error| {
        failed(format!(
            "E_TOPIC_RESEARCH_RUN_LEDGER: could not write {}: {error}",
            run_ledger_path(set_root).display()
        ))
    })
}

fn printed_warnings(
    notices: &[String],
    replaced_run: Option<&ReplacedRun>,
    plan: &RoundPlan,
) -> String {
    let replaced_notice = replaced_run.map(ledger_wording::replaced_notice);
    let trims = plan.trims.iter().map(plan_wording::trim);
    warning_lines(notices.iter().cloned().chain(replaced_notice).chain(trims))
}

fn warning_lines(warnings: impl Iterator<Item = String>) -> String {
    let mut stderr = String::new();
    for warning in warnings {
        let _ = writeln!(stderr, "warning: {warning}");
    }
    stderr
}

fn budgeted(
    ledger: RunLedger,
    plan: &RoundPlan,
    search_cap: &EnvironmentSearchCap,
    notices: &mut Vec<String>,
) -> RunLedger {
    if !ledger.awaits_search_budgets() {
        return ledger;
    }
    let budgets =
        SearchBudgets::sized(&unsearched_tree(&plan.pairs), search_cap.cap);
    if budgets.overruns(search_cap.cap) {
        notices.push(search_wording::search_floor(&budgets, search_cap));
    }
    ledger.budgeting_searches(budgets)
}

fn unhonoured_search_cap_notice(
    search_cap: &EnvironmentSearchCap,
    plan: &RoundPlan,
) -> Option<String> {
    let value = search_cap.unhonoured.as_deref()?;
    let has_web_nodes = !unsearched_tree(&plan.pairs).is_empty();
    has_web_nodes
        .then(|| search_wording::unhonoured_search_cap(value, search_cap.cap))
}

fn start_notices(concurrency: &CappedConcurrency) -> Vec<String> {
    let unhonoured = concurrency
        .cap
        .unhonoured
        .iter()
        .map(concurrency_wording::unhonoured_cap);
    let clamps = concurrency.resolved.warnings.iter().map(|warning| {
        concurrency_wording::concurrency_warning(
            warning,
            concurrency.cap.source,
        )
    });
    unhonoured.chain(clamps).collect()
}

enum ReplacedLedger {
    Absent,
    Readable(Box<RunLedger>),
    Corrupt,
}

fn replaced_ledger<F: FileReader>(
    set_root: &Path,
    fs: &F,
) -> Result<ReplacedLedger, String> {
    match read_run_ledger(set_root, fs, &UnicodeTables) {
        Ok(None) => Ok(ReplacedLedger::Absent),
        Ok(Some(stored)) => Ok(ReplacedLedger::Readable(Box::new(stored))),
        Err(LedgerError::Corrupt) => Ok(ReplacedLedger::Corrupt),
        Err(LedgerError::Unreadable(error)) => Err(error.to_string()),
    }
}

fn superseded(run: &RunId, owner: &RunId) -> String {
    failed(format!(
        "E_TOPIC_RESEARCH_RUN_SUPERSEDED: run {run} was superseded by run \
         {owner}; another conduct run now owns this set, so let it finish"
    ))
}

fn no_usable_ledger(run: &RunId) -> String {
    failed(format!(
        "E_TOPIC_RESEARCH_RUN_LEDGER: no usable ledger for run {run}; re-run \
         conduct to start a fresh run"
    ))
}

fn unfinished_spawn(unfinished: &UnfinishedSpawn) -> Value {
    json!({
        "spawn": unfinished.spawn.to_string(),
        "rejected": unfinished.rejected.as_ref().map(plan_wording::rejection),
    })
}

fn trim(trim: &Trim) -> Value {
    json!({
        "stem": trim.at.stem.to_string(),
        "lineage": trim.at.lineage.to_string(),
        "recorded": trim.recorded,
        "cap": trim.cap,
    })
}

fn unanswered(outstanding: &OutstandingPair) -> Value {
    json!({
        "stem": outstanding.stem.to_string(),
        "question": outstanding.pair.question,
        "profile": outstanding.pair.profile,
        "stage": stage_name(&outstanding.stage),
    })
}

const fn stage_name(stage: &Stage) -> &'static str {
    match stage {
        Stage::SinglePass => "single_pass",
        Stage::Compose(_) => "compose",
        Stage::ResearchNodes(_) => "research_nodes",
    }
}

fn search_budgets(budgets: &SearchBudgets) -> Value {
    budgets
        .levels()
        .map(|(level, budget)| json!({"level": level, "budget": budget}))
        .collect()
}

fn shallow_finding(finding: &ShallowFinding) -> Value {
    json!({
        "stem": finding.stem.to_string(),
        "depth": finding.depth.levels(),
    })
}

struct PlanOutput<'a> {
    plan: &'a RoundPlan,
    window: &'a Window,
    set_root: &'a Path,
    request: &'a PlanRequest,
    ledger: Option<&'a RunLedger>,
    replaced_run: Option<&'a ReplacedRun>,
    warnings: Vec<String>,
}

impl PlanOutput<'_> {
    fn render(&self) -> Value {
        let plan = self.plan;
        let items: Vec<_> = plan
            .items
            .iter()
            .map(|item| {
                json!({
                    "line": item.line,
                    "question": item.question,
                    "complete": item.complete,
                })
            })
            .collect();
        let pairs: Vec<_> = self
            .window
            .pairs
            .iter()
            .map(|pair| self.pair(pair))
            .collect();
        let skipped: Vec<_> = plan
            .skipped
            .iter()
            .map(|skip| {
                json!({
                    "question": skip.pair.question,
                    "profile": skip.pair.profile,
                    "reason": plan_wording::skip_reason(skip),
                })
            })
            .collect();
        let unfinished: Vec<_> = self
            .window
            .unfinished
            .iter()
            .map(unfinished_spawn)
            .collect();
        let trims: Vec<_> = plan.trims.iter().map(trim).collect();
        let shallow: Vec<_> =
            plan.shallow.iter().map(shallow_finding).collect();
        let PlanRequest {
            depth,
            concurrency,
            search_cap,
            ..
        } = self.request;
        let mut rendered = json!({
            "items": items,
            "pairs": pairs,
            "skipped": skipped,
            "warnings": self.warnings,
            "depth": depth.levels(),
            "concurrency": concurrency.resolved.concurrency.agents().get(),
            "subagent_cap": {
                "slots": concurrency.cap.cap.slots().get(),
                "source": concurrency.cap.source.reported_as(),
            },
            "search_cap": {
                "searches": search_cap.cap.searches().get(),
                "source": search_cap.source.reported_as(),
            },
            "remaining": self.window.remaining,
            "unfinished": unfinished,
            "trims": trims,
            "shallow": shallow,
            "unanswered": plan.pairs.iter().map(unanswered).collect::<Vec<_>>(),
        });
        if let Some(ledger) = self.ledger {
            rendered["run"] = json!(ledger.run().to_string());
            rendered["batch"] = json!(ledger.memory().pending.number());
            rendered["withheld"] =
                ledger.memory().withheld.profiles().collect();
            rendered["unexpected"] = self
                .window
                .unexpected
                .iter()
                .map(ToString::to_string)
                .collect();
            if let Some(budgets) = ledger
                .search_budgets()
                .filter(|budgets| budgets.levels().next().is_some())
            {
                rendered["search_budgets"] = search_budgets(budgets);
                rendered["search_demand"] = json!(budgets.demand());
            }
        }
        if let Some(replaced) = self.replaced_run {
            rendered["replaced_run"] = self.leftovers(replaced);
        }
        rendered
    }

    fn leftovers(&self, replaced: &ReplacedRun) -> Value {
        let entries = |leftovers: &[Leftover]| -> Vec<Value> {
            leftovers
                .iter()
                .map(|leftover| {
                    json!({
                        "spawn": leftover.spawn().to_string(),
                        "question": leftover.pair().map(|pair| &pair.question),
                        "profile": leftover.pair().map(|pair| &pair.profile),
                    })
                })
                .collect()
        };
        json!({
            "run": replaced.run().to_string(),
            "valid": entries(replaced.valid()),
            "invalid": entries(replaced.invalid()),
            "unwritten": entries(replaced.unwritten()),
            "unasked": entries(replaced.unasked()),
            "quarantine": replaced
                .quarantine()
                .iter()
                .map(|stem| self.absolute(&stem.finding_path()))
                .collect::<Vec<_>>(),
            "owed": replaced
                .owed()
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>(),
        })
    }

    fn pair(&self, outstanding: &OutstandingPair) -> Value {
        let mut rendered = json!({
            "question": outstanding.pair.question,
            "profile": outstanding.pair.profile,
            "path": self.absolute(&outstanding.stem.finding_path()),
            "stage": stage_name(&outstanding.stage),
        });
        let spawn = SpawnRef::Pair(outstanding.stem.clone()).to_string();
        let note = |lineage: &Lineage| NoteRef {
            stem: outstanding.stem.clone(),
            lineage: lineage.clone(),
        };
        let search_budget = |level| self.search_budget(outstanding, level);
        match &outstanding.stage {
            Stage::SinglePass => {
                rendered["spawn"] = json!(spawn);
                if let Some(budget) = search_budget(1) {
                    rendered["search_budget"] = json!(budget);
                }
            }
            Stage::Compose(lineages) => {
                rendered["notes"] = lineages
                    .iter()
                    .map(|lineage| json!(self.absolute(&note(lineage).path())))
                    .collect();
                rendered["spawn"] = json!(spawn);
            }
            Stage::ResearchNodes(nodes) => {
                rendered["nodes"] = nodes
                    .iter()
                    .map(|node| {
                        self.node(
                            note(&node.lineage),
                            node,
                            search_budget(node.lineage.level()),
                        )
                    })
                    .collect();
            }
        }
        rendered
    }

    fn search_budget(
        &self,
        outstanding: &OutstandingPair,
        level: u32,
    ) -> Option<NonZeroU32> {
        let budgets = self.ledger?.search_budgets()?;
        (outstanding.pair.profile == SEARCHING_PROFILE)
            .then(|| budgets.at(level))
    }

    fn node(
        &self,
        address: NoteRef,
        node: &MissingNode,
        search_budget: Option<NonZeroU32>,
    ) -> Value {
        let mut rendered = json!({
            "lineage": node.lineage.to_string(),
            "level": node.lineage.level(),
            "question": node.question,
            "cap": node.cap(),
            "id": address.id(&self.set_slug()),
            "path": self.absolute(&address.path()),
            "known_questions": node.known_questions,
            "spawn": SpawnRef::Node(address).to_string(),
        });
        if let Some(rejected) = &node.rejected {
            rendered["rejected"] = json!(plan_wording::rejection(rejected));
        }
        if let Some(budget) = search_budget {
            rendered["search_budget"] = json!(budget);
        }
        rendered
    }

    fn absolute(&self, relative: &str) -> String {
        self.set_root.join(relative).to_string_lossy().into_owned()
    }

    fn set_slug(&self) -> String {
        self.set_root
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroU32;

    use research::conduct::concurrency::ConcurrencyWarning;
    use research::conduct::concurrency::ConfigFile;
    use research::conduct::concurrency::Provenance;
    use research::conduct::concurrency::RequestedConcurrency;
    use research::conduct::concurrency::SubagentCap;

    use super::resolve_concurrency;
    use crate::subagent_cap::CapSource;
    use crate::subagent_cap::EnvironmentCap;

    fn cap(slots: u32) -> EnvironmentCap {
        EnvironmentCap {
            cap: SubagentCap::new(
                NonZeroU32::new(slots).unwrap_or(NonZeroU32::MIN),
            ),
            source: CapSource::Variable("ACCELERATOR_SUBAGENT_CAP"),
            unhonoured: None,
        }
    }

    fn requested(value: &str, provenance: Provenance) -> RequestedConcurrency {
        RequestedConcurrency::Given {
            value: value.to_owned(),
            provenance,
        }
    }

    fn configured(
        value: &str,
    ) -> impl FnOnce() -> Result<RequestedConcurrency, String> + '_ {
        move || Ok(requested(value, Provenance::ConfigFile(ConfigFile::Team)))
    }

    fn unreadable() -> Result<RequestedConcurrency, String> {
        Err("unreadable".to_owned())
    }

    fn agents(
        flag: Option<RequestedConcurrency>,
        configured: impl FnOnce() -> Result<RequestedConcurrency, String>,
        cap: EnvironmentCap,
    ) -> Result<u32, String> {
        resolve_concurrency(flag, configured, cap)
            .map(|capped| capped.resolved.concurrency.agents().get())
    }

    #[test]
    fn a_concurrency_flag_never_reads_the_configured_value() {
        assert_eq!(
            agents(Some(requested("5", Provenance::Flag)), unreadable, cap(20)),
            Ok(5)
        );
    }

    #[test]
    fn without_a_flag_the_configured_value_is_resolved() {
        assert_eq!(agents(None, configured("24"), cap(30)), Ok(24));
    }

    #[test]
    fn a_lowered_cap_bounds_the_configured_value() {
        let cap = cap(8);
        let resolved = resolve_concurrency(None, configured("24"), cap.clone());

        assert_eq!(
            resolved
                .as_ref()
                .map(|r| r.resolved.concurrency.agents().get()),
            Ok(8)
        );
        assert_eq!(
            resolved.map(|r| r.resolved.warnings),
            Ok(vec![ConcurrencyWarning::AboveCap {
                given: "24".to_owned(),
                provenance: Provenance::ConfigFile(ConfigFile::Team),
                cap: cap.cap,
            }])
        );
    }

    #[test]
    fn an_empty_concurrency_names_its_source_in_the_error() {
        assert_eq!(
            agents(Some(requested("", Provenance::Flag)), unreadable, cap(20)),
            Err(
                "E_TOPIC_RESEARCH_CONCURRENCY: --concurrency is empty; pass \
                 a positive integer"
                    .to_owned()
            )
        );
    }

    #[test]
    fn a_configured_value_that_cannot_be_read_is_an_error() {
        assert_eq!(
            agents(None, unreadable, cap(20)),
            Err("unreadable".to_owned())
        );
    }
}
