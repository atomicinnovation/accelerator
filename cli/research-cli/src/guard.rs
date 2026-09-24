//! The `research guard` `PreToolUse` hook: confines the researcher subagent
//! to the fetch and to finding files.
//!
//! Exit `2` blocks the call; every other outcome exits `0`, because a hook
//! that fails in any other way must not block unrelated work.

use std::cell::OnceCell;
use std::io::Read as _;
use std::path::Path;
use std::path::PathBuf;
use std::process::ExitCode;

use config::catalogue;
use config::ConfigError;
use config::Key;
use config_adapters::FileConfigStore;
use research::confinement::confined_researcher;
use research::confinement::decide;
use research::confinement::Action;
use research::confinement::Decision;
use research::confinement::InternalFailure;
use research::confinement::Refusal;
use research::confinement::Researcher;
use research::confinement::Researchers;
use research::confinement::ToolCall;
use research::confinement::DEFAULT_RESEARCHER;
use serde::Deserialize;
use serde_json::value::RawValue;
use serde_json::Value;

use crate::context::ProjectContext;
use crate::write_target::locate;
use crate::write_target::CorpusFindings;

const BLOCK: u8 = 2;
const TOPICS_KEY: &str = "paths.research_topics";

/// Only the fields Claude Code itself writes. `tool_input` stays raw, since
/// the researcher controls its strings and a value that fails to parse
/// there must fail only the confined call, not the whole envelope.
#[derive(Deserialize)]
struct Envelope {
    agent_id: Option<String>,
    agent_type: Option<String>,
    tool_name: Option<String>,
    cwd: Option<PathBuf>,
    tool_input: Option<Box<RawValue>>,
}

pub fn run() -> ExitCode {
    let mut input = String::new();
    if let Err(error) = std::io::stdin().read_to_string(&mut input) {
        return pass_with(&format!("could not read the hook input: {error}"));
    }
    let envelope: Envelope = match serde_json::from_str(&input) {
        Ok(envelope) => envelope,
        Err(error) => {
            return pass_with(&format!("unreadable hook input: {error}"));
        }
    };
    let call =
        ToolCall::new(envelope.agent_id.clone(), envelope.agent_type.clone());
    if !call.is_subagent() {
        return ExitCode::SUCCESS;
    }
    let Some(cwd) = envelope
        .cwd
        .clone()
        .or_else(|| std::env::current_dir().ok())
    else {
        return pass_with("no working directory to judge the call from");
    };
    let project = Project::at(cwd);
    let Some(researcher) =
        confined_researcher(&call, &project.researchers(call.agent_type()))
    else {
        return ExitCode::SUCCESS;
    };
    block_on_panic(&researcher);
    #[cfg(feature = "test-loopback")]
    crate::loopback::panic_if_asked();
    judge(&envelope, &project, &researcher)
}

fn judge(
    envelope: &Envelope,
    project: &Project,
    researcher: &Researcher,
) -> ExitCode {
    let action = action(envelope, project);
    match decide(&action, &CorpusFindings) {
        Decision::Pass => ExitCode::SUCCESS,
        Decision::Block(block) => {
            let refusal = Refusal {
                researcher,
                topics: &project.topics().display().to_string(),
                block: &block,
            };
            eprintln!("{refusal}");
            ExitCode::from(BLOCK)
        }
    }
}

fn action(envelope: &Envelope, project: &Project) -> Action {
    let Some(tool_input) = envelope
        .tool_input
        .as_deref()
        .and_then(|raw| serde_json::from_str::<Value>(raw.get()).ok())
    else {
        return Action::Unreadable;
    };
    let field = match envelope.tool_name.as_deref() {
        Some("Bash") => "command",
        Some("Write" | "Edit" | "MultiEdit") => "file_path",
        Some("NotebookEdit") => "notebook_path",
        _ => return Action::Unreadable,
    };
    let Some(text) = tool_input.get(field).and_then(Value::as_str) else {
        return Action::Unreadable;
    };
    if field == "command" {
        Action::Command(text.to_owned())
    } else {
        Action::Write(locate(text, &project.cwd, project.topics()))
    }
}

/// A panic would exit `101`, which Claude Code reads as non-blocking, and
/// the lexer judges researcher-written text. A hook rather than
/// `catch_unwind` also holds under `panic = "abort"`.
fn block_on_panic(researcher: &Researcher) {
    let reason = InternalFailure(researcher).to_string();
    std::panic::set_hook(Box::new(move |_| {
        eprintln!("{reason}");
        std::process::exit(i32::from(BLOCK));
    }));
}

fn pass_with(diagnostic: &str) -> ExitCode {
    eprintln!("accelerator research guard: {diagnostic}; not judging the call");
    ExitCode::SUCCESS
}

/// The project a call runs in, whose configuration is read at most once and
/// only when a judgement needs it.
struct Project {
    cwd: PathBuf,
    context: OnceCell<Option<ProjectContext>>,
    topics: OnceCell<PathBuf>,
}

impl Project {
    const fn at(cwd: PathBuf) -> Self {
        Self {
            cwd,
            context: OnceCell::new(),
            topics: OnceCell::new(),
        }
    }

    fn context(&self) -> Option<&ProjectContext> {
        self.context
            .get_or_init(|| {
                read_or_default(ProjectContext::enclosing(&self.cwd))
            })
            .as_ref()
    }

    fn researchers(&self, agent_type: &str) -> Researchers {
        if agent_type == DEFAULT_RESEARCHER {
            return Researchers::default_only();
        }
        self.context()
            .and_then(|context| {
                read_or_default(catalogue::agent_name(
                    context.config.as_ref(),
                    "researcher",
                ))
            })
            .map_or_else(Researchers::default_only, |name| {
                Researchers::with_configured(&name)
            })
    }

    fn topics(&self) -> &Path {
        self.topics.get_or_init(|| {
            let Some(context) = self.context() else {
                return FileConfigStore::discover_root(&self.cwd)
                    .join(default_topics());
            };
            let configured = read_or_default(configured_topics(context))
                .unwrap_or_else(default_topics);
            context.root.join(configured)
        })
    }
}

/// Unreadable configuration never stops the guard confining the
/// researcher: it falls back to the catalogue defaults and says so.
fn read_or_default<T>(read: Result<T, ConfigError>) -> Option<T> {
    read.map_err(|error| {
        let error = error.to_string();
        let summary = error.lines().next().unwrap_or_default();
        eprintln!(
            "accelerator research guard: could not read the configuration, \
             using the defaults: {summary}"
        );
    })
    .ok()
}

fn configured_topics(context: &ProjectContext) -> Result<String, ConfigError> {
    let key = Key::parse(TOPICS_KEY)?;
    Ok(context.config.effective_nonempty(&key, None)?.rendered())
}

fn default_topics() -> String {
    catalogue::default_for(TOPICS_KEY)
        .map(|value| config::render_value(&value))
        .unwrap_or_default()
}
