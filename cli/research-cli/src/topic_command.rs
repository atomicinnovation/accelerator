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
use corpus::FileRemove;
use corpus_adapters::resolve::resolve_document;
use corpus_adapters::resolve::Resolution;
use research::round::Pair;
use research::round::Round;
use research::round::Stage;
use research::run_ledger::Continuation;
use research::run_ledger::RunId;
use research::run_ledger::RunLedger;
use research::spawn_window::window;
use research::spawn_window::SpawnRef;
use research::spawn_window::Window;
use research::tree::Depth;
use research::tree::Node;
use research::tree::NoteRef;
use research_adapters::topic_research::read_round_inputs;
use research_adapters::topic_research::run_ledger::delete_run_ledger;
use research_adapters::topic_research::run_ledger::read_run_ledger;
use research_adapters::topic_research::run_ledger::run_ledger_path;
use research_adapters::topic_research::run_ledger::write_run_ledger;
use research_adapters::topic_research::run_ledger::LedgerError;
use research_adapters::unicode_text::UnicodeTables;
use serde_json::json;
use serde_json::Value;

use crate::context::ProjectContext;

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
/// A message when `slug` resolves to no set.
pub fn resolve_set(
    project: &ProjectContext,
    cwd: &Path,
    slug: &str,
) -> Result<PathBuf, String> {
    let Resolution::Resolved(set_root) =
        resolve_document(cwd, DOC_TYPE, slug, |key| project.type_dir(key))
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
    pub limit: Option<usize>,
    pub run: RunFlag,
}

impl OutstandingFlags {
    /// # Errors
    ///
    /// A message naming the first flag that is malformed or
    /// combined with one it excludes.
    pub fn parse(
        depth: &str,
        limit: Option<&str>,
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
        let limit = limit
            .map(|limit| {
                positive(limit)
                    .and_then(|limit| usize::try_from(limit).ok())
                    .ok_or_else(|| {
                        failed(format!(
                            "E_TOPIC_RESEARCH_LIMIT: --limit must be a \
                             positive integer, got '{limit}'"
                        ))
                    })
            })
            .transpose()?;
        Ok(Self {
            depth,
            limit,
            run: run_flag(start, run, spawned)?,
        })
    }
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
    limit: Option<usize>,
    mode: RunMode,
    fs: &F,
    store: &S,
) -> Result<Printed, String>
where
    F: DirReader + FileReader + DirectoryProbe,
    S: AtomicWrite,
{
    let mut notices = Vec::new();
    let continued = match &mode {
        RunMode::Continue { run, spawned } => {
            Some(continued_ledger(set_root, run, *spawned, fs)?)
        }
        RunMode::Start(_) => {
            notices.extend(replaced_ledger_notice(set_root, fs)?);
            None
        }
        RunMode::Query => None,
    };
    let pins = continued
        .as_ref()
        .map(|ledger| ledger.pins().clone())
        .unwrap_or_default();
    let reading = read_round_inputs(
        set_root,
        profiles_dir,
        depth,
        pins,
        fs,
        &UnicodeTables,
    )
    .map_err(|error| error.to_string())?;
    let round = Round::plan(&reading.inputs, &UnicodeTables);
    let ledger = match mode {
        RunMode::Start(run) => Some(RunLedger::start(run, &round)),
        RunMode::Continue { .. } => continued,
        RunMode::Query => None,
    };
    let window = window(round, limit, ledger.as_ref().map(RunLedger::attempts));
    let ledger = ledger.map(|ledger| ledger.record(&window));
    if let Some(ledger) = &ledger {
        write_run_ledger(set_root, ledger, store).map_err(|error| {
            failed(format!(
                "E_TOPIC_RESEARCH_RUN_LEDGER: could not write {}: {error}",
                run_ledger_path(set_root).display()
            ))
        })?;
    }
    let mut stderr = String::new();
    let trims = window.round.trims.iter().map(ToString::to_string);
    for warning in notices.iter().cloned().chain(trims) {
        let _ = writeln!(stderr, "warning: {warning}");
    }
    let warnings = window
        .round
        .warnings
        .iter()
        .map(ToString::to_string)
        .chain(reading.warnings.iter().map(ToString::to_string))
        .chain(notices)
        .collect();
    let plan = Plan {
        window: &window,
        set_root,
        depth,
        ledger: ledger.as_ref(),
        warnings,
    };
    Ok(Printed {
        stdout: format!("{}\n", plan.render()),
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
        Ok(Some(stored)) => {
            match RunLedger::continue_as(stored, run, spawned) {
                Continuation::Continued(ledger) => Ok(ledger),
                Continuation::Superseded { stored } => {
                    Err(superseded(run, &stored))
                }
            }
        }
        Ok(None) | Err(LedgerError::Corrupt) => Err(no_usable_ledger(run)),
        Err(LedgerError::Unreadable(error)) => Err(error.to_string()),
    }
}

fn replaced_ledger_notice<F: FileReader>(
    set_root: &Path,
    fs: &F,
) -> Result<Option<String>, String> {
    const CONSEQUENCE: &str = "stops at its next batch, and notes its last \
                               batch writes may show as unexpected";
    match read_run_ledger(set_root, fs, &UnicodeTables) {
        Ok(None) => Ok(None),
        Ok(Some(stored)) => Ok(Some(format!(
            "replaced the ledger of run {}; if that run is still going it \
             {CONSEQUENCE}",
            stored.run()
        ))),
        Err(LedgerError::Corrupt) => Ok(Some(format!(
            "replaced a corrupt run ledger; if its run is still going it \
             {CONSEQUENCE}"
        ))),
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

struct Plan<'a> {
    window: &'a Window,
    set_root: &'a Path,
    depth: Depth,
    ledger: Option<&'a RunLedger>,
    warnings: Vec<String>,
}

impl Plan<'_> {
    fn render(&self) -> Value {
        let round = &self.window.round;
        let items: Vec<_> = round
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
        let pairs: Vec<_> =
            round.pairs.iter().map(|pair| self.pair(pair)).collect();
        let skipped: Vec<_> = round
            .skipped
            .iter()
            .map(|skip| {
                json!({
                    "question": skip.question,
                    "profile": skip.profile,
                    "reason": skip.explanation(),
                })
            })
            .collect();
        let unaccepted: Vec<_> = self
            .window
            .unaccepted
            .iter()
            .map(|unaccepted| {
                json!({
                    "spawn": unaccepted.spawn.to_string(),
                    "rejected": unaccepted
                        .rejected
                        .as_ref()
                        .map(ToString::to_string),
                })
            })
            .collect();
        let trims: Vec<_> = round
            .trims
            .iter()
            .map(|trim| {
                json!({
                    "stem": trim.stem.to_string(),
                    "lineage": trim.trim.lineage.to_string(),
                    "recorded": trim.trim.recorded,
                    "cap": trim.trim.cap,
                })
            })
            .collect();
        let shallower: Vec<_> = round
            .shallower
            .iter()
            .map(|pair| {
                json!({
                    "stem": pair.stem.to_string(),
                    "depth": pair.depth.levels(),
                })
            })
            .collect();
        let mut plan = json!({
            "items": items,
            "pairs": pairs,
            "skipped": skipped,
            "warnings": self.warnings,
            "depth": self.depth.levels(),
            "remaining": self.window.remaining,
            "unaccepted": unaccepted,
            "trims": trims,
            "shallower": shallower,
        });
        if let Some(ledger) = self.ledger {
            plan["run"] = json!(ledger.run().to_string());
            plan["batch"] = json!(ledger.pending().number());
            plan["unexpected"] = self
                .window
                .unexpected
                .iter()
                .map(ToString::to_string)
                .collect();
        }
        plan
    }

    fn pair(&self, pair: &Pair) -> Value {
        let mut rendered = json!({
            "question": pair.question,
            "profile": pair.profile,
            "path": self.absolute(&pair.path),
        });
        let spawn = SpawnRef::Pair(pair.stem.clone()).to_string();
        match &pair.stage {
            Stage::Research => {
                rendered["stage"] = json!("research");
                rendered["spawn"] = json!(spawn);
            }
            Stage::Compose(lineages) => {
                rendered["stage"] = json!("compose");
                rendered["notes"] = lineages
                    .iter()
                    .map(|lineage| {
                        json!(self.absolute(&pair.level_note_path(lineage)))
                    })
                    .collect();
                rendered["spawn"] = json!(spawn);
            }
            Stage::Deepen(nodes) => {
                rendered["stage"] = json!("deepen");
                rendered["nodes"] =
                    nodes.iter().map(|node| self.node(pair, node)).collect();
            }
        }
        rendered
    }

    fn node(&self, pair: &Pair, node: &Node) -> Value {
        let spawn = SpawnRef::Node(NoteRef {
            stem: pair.stem.clone(),
            lineage: node.lineage.clone(),
        });
        let mut rendered = json!({
            "lineage": node.lineage.to_string(),
            "level": node.lineage.level(),
            "question": node.question,
            "cap": node.cap,
            "id": pair.level_note_id(&node.lineage, &self.set_slug()),
            "path": self.absolute(&pair.level_note_path(&node.lineage)),
            "known_questions": node.known_questions,
            "spawn": spawn.to_string(),
        });
        if let Some(rejected) = &node.rejected {
            rendered["rejected"] = json!(rejected.to_string());
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
