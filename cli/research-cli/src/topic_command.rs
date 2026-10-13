//! The `research topic` command layer: a set's round plan rendered as the
//! JSON `research-topic`'s `conduct` consumes, and the run ledger `conduct`
//! keeps between plans.

use std::fmt::Write as _;
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
use research::conduct::concurrency::Concurrency;
use research::conduct::concurrency::Provenance;
use research::conduct::concurrency::RequestedConcurrency;
use research::conduct::concurrency::ResolvedConcurrency;
use research::conduct::ledger::RunId;
use research::conduct::ledger::RunLedger;
use research::conduct::observed::Observed;
use research::conduct::replaced_run::Leftover;
use research::conduct::replaced_run::ReplacedBatch;
use research::conduct::replaced_run::ReplacedRun;
use research::conduct::spawn::SpawnRef;
use research::conduct::window::window;
use research::conduct::window::Window;
use research::topic::layout::lineage::Lineage;
use research::topic::layout::note_ref::NoteRef;
use research::topic::plan::OutstandingPair;
use research::topic::plan::RoundInputs;
use research::topic::plan::RoundPlan;
use research::topic::tree::Depth;
use research::topic::tree::MissingNode;
use research::topic::tree::Stage;
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
    Continue { run: RunId, spawned: Option<u32> },
}

/// The run flags as given, before a started run is minted its id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunFlag {
    Query,
    Start,
    Continue { run: RunId, spawned: Option<u32> },
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
    ) -> Result<Self, String> {
        let depth = positive(depth).and_then(Depth::new).ok_or_else(|| {
            failed(format!(
                "E_TOPIC_RESEARCH_DEPTH: --depth must be a positive integer, \
                 got '{depth}'"
            ))
        })?;
        Ok(Self {
            depth,
            requested_concurrency: concurrency.map(|value| {
                RequestedConcurrency::Given {
                    value: value.to_owned(),
                    provenance: Provenance::Flag,
                }
            }),
            run: run_flag(start, run, spawned)?,
        })
    }
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
    })
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

fn positive(text: &str) -> Option<u32> {
    non_negative(text).filter(|value| *value > 0)
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
    depth: Depth,
    concurrency: &CappedConcurrency,
    mode: RunMode,
    fs: &F,
    store: &S,
) -> Result<Printed, String>
where
    F: DirReader + FileReader + DirectoryProbe,
    S: AtomicWrite,
{
    let mut notices = Vec::new();
    let (continued, replaced) = match &mode {
        RunMode::Continue { run, spawned } => (
            Some(continued_ledger(set_root, run, *spawned, fs)?),
            ReplacedLedger::Absent,
        ),
        RunMode::Start(_) => {
            notices.extend(start_notices(concurrency));
            let replaced = replaced_ledger(set_root, fs)?;
            notices.extend(ledger_wording::corrupt_notice(&replaced));
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
        depth,
        claims,
        fs,
        &YamlFrontmatter,
        &UnicodeTables,
    )
    .map_err(|error| error.to_string())?;
    let replaced_batch = replaced.judging_findings(&reading.inputs);
    let inputs = match &replaced_batch {
        Some(replaced_batch) => {
            replaced_batch.quarantining(reading.inputs, &UnicodeTables)
        }
        None => reading.inputs,
    };
    let plan = RoundPlan::of(&inputs, &UnicodeTables);
    let replaced_run = replaced_batch.map(|replaced_batch| {
        replaced_batch.report(&inputs, &plan, &UnicodeTables)
    });
    let observed = Observed::of(&inputs);
    let ledger = match mode {
        RunMode::Start(run) => Some(RunLedger::start(run, &observed)),
        RunMode::Continue { .. } => continued,
        RunMode::Query => None,
    };
    let window = window(
        &plan,
        &observed,
        concurrency.resolved.concurrency,
        ledger.as_ref().map(RunLedger::memory),
    );
    let ledger = ledger.map(|ledger| ledger.record(&plan, &observed, &window));
    if let Some(ledger) = &ledger {
        write_run_ledger(set_root, ledger, store).map_err(|error| {
            failed(format!(
                "E_TOPIC_RESEARCH_RUN_LEDGER: could not write {}: {error}",
                run_ledger_path(set_root).display()
            ))
        })?;
    }
    let mut stderr = String::new();
    let replaced_notice =
        replaced_run.as_ref().map(ledger_wording::replaced_notice);
    let trims = plan.trims.iter().map(plan_wording::trim);
    for warning in notices.iter().cloned().chain(replaced_notice).chain(trims) {
        let _ = writeln!(stderr, "warning: {warning}");
    }
    let warnings = plan
        .warnings
        .iter()
        .map(plan_wording::warning)
        .chain(reading.warnings.iter().map(ToString::to_string))
        .chain(notices)
        .collect();
    let rendered = PlanOutput {
        plan: &plan,
        window: &window,
        set_root,
        depth,
        concurrency: concurrency.resolved.concurrency,
        cap: &concurrency.cap,
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

pub enum ReplacedLedger {
    Absent,
    Readable(RunLedger),
    Corrupt,
}

fn replaced_ledger<F: FileReader>(
    set_root: &Path,
    fs: &F,
) -> Result<ReplacedLedger, String> {
    match read_run_ledger(set_root, fs, &UnicodeTables) {
        Ok(None) => Ok(ReplacedLedger::Absent),
        Ok(Some(stored)) => Ok(ReplacedLedger::Readable(stored)),
        Err(LedgerError::Corrupt) => Ok(ReplacedLedger::Corrupt),
        Err(LedgerError::Unreadable(error)) => Err(error.to_string()),
    }
}

impl ReplacedLedger {
    fn judging_findings(&self, inputs: &RoundInputs) -> Option<ReplacedBatch> {
        match self {
            Self::Readable(stored) => Some(ReplacedBatch::judging_findings(
                stored,
                inputs,
                &UnicodeTables,
            )),
            Self::Absent | Self::Corrupt => None,
        }
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

struct PlanOutput<'a> {
    plan: &'a RoundPlan,
    window: &'a Window,
    set_root: &'a Path,
    depth: Depth,
    concurrency: Concurrency,
    cap: &'a EnvironmentCap,
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
            .map(|unfinished| {
                json!({
                    "spawn": unfinished.spawn.to_string(),
                    "rejected": unfinished
                        .rejected
                        .as_ref()
                        .map(plan_wording::rejection),
                })
            })
            .collect();
        let trims: Vec<_> = plan
            .trims
            .iter()
            .map(|trim| {
                json!({
                    "stem": trim.at.stem.to_string(),
                    "lineage": trim.at.lineage.to_string(),
                    "recorded": trim.recorded,
                    "cap": trim.cap,
                })
            })
            .collect();
        let shallow: Vec<_> = plan
            .shallow
            .iter()
            .map(|finding| {
                json!({
                    "stem": finding.stem.to_string(),
                    "depth": finding.depth.levels(),
                })
            })
            .collect();
        let mut rendered = json!({
            "items": items,
            "pairs": pairs,
            "skipped": skipped,
            "warnings": self.warnings,
            "depth": self.depth.levels(),
            "concurrency": self.concurrency.agents().get(),
            "subagent_cap": {
                "slots": self.cap.cap.slots().get(),
                "source": self.cap.source.reported_as(),
            },
            "remaining": self.window.remaining,
            "unfinished": unfinished,
            "trims": trims,
            "shallow": shallow,
        });
        if let Some(ledger) = self.ledger {
            rendered["run"] = json!(ledger.run().to_string());
            rendered["batch"] = json!(ledger.memory().pending.number());
            rendered["unexpected"] = self
                .window
                .unexpected
                .iter()
                .map(ToString::to_string)
                .collect();
        }
        if let Some(replaced) = self.replaced_run {
            rendered["replaced_run"] = self.replaced_run(replaced);
        }
        rendered
    }

    fn replaced_run(&self, replaced: &ReplacedRun) -> Value {
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
        })
    }

    fn pair(&self, outstanding: &OutstandingPair) -> Value {
        let mut rendered = json!({
            "question": outstanding.pair.question,
            "profile": outstanding.pair.profile,
            "path": self.absolute(&outstanding.stem.finding_path()),
        });
        let spawn = SpawnRef::Pair(outstanding.stem.clone()).to_string();
        let note = |lineage: &Lineage| NoteRef {
            stem: outstanding.stem.clone(),
            lineage: lineage.clone(),
        };
        match &outstanding.stage {
            Stage::SinglePass => {
                rendered["stage"] = json!("single_pass");
                rendered["spawn"] = json!(spawn);
            }
            Stage::Compose(lineages) => {
                rendered["stage"] = json!("compose");
                rendered["notes"] = lineages
                    .iter()
                    .map(|lineage| json!(self.absolute(&note(lineage).path())))
                    .collect();
                rendered["spawn"] = json!(spawn);
            }
            Stage::ResearchNodes(nodes) => {
                rendered["stage"] = json!("research_nodes");
                rendered["nodes"] = nodes
                    .iter()
                    .map(|node| self.node(note(&node.lineage), node))
                    .collect();
            }
        }
        rendered
    }

    fn node(&self, address: NoteRef, node: &MissingNode) -> Value {
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
