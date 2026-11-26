//! The `research guard` `PreToolUse` hook: confines the researcher subagent
//! to the fetch, findings and level notes, and the composer subagent to
//! findings alone.
//!
//! Exit `2` blocks the call, as it does a subagent's call the guard cannot
//! tell is unconfined. Every other outcome exits `0`, because a hook that
//! fails in any other way must not block unrelated work.

use std::cell::OnceCell;
use std::io::Read as _;
use std::path::Path;
use std::path::PathBuf;
use std::process::ExitCode;

use config::catalogue;
use config::ConfigError;
use config::Key;
use config_adapters::FileConfigStore;
use research::confinement::confined_agent;
use research::confinement::decide;
use research::confinement::Action;
use research::confinement::ConfinedAgent;
use research::confinement::Confinement;
use research::confinement::Decision;
use research::confinement::InternalFailure;
use research::confinement::Refusal;
use research::confinement::ToolCall;
use research::confinement::Unjudgeable;
use research::confinement::UnjudgedCall;
use research::confinement::DEFAULT_COMPOSER;
use research::confinement::DEFAULT_RESEARCHER;
use serde::Deserialize;
use serde_json::value::RawValue;
use serde_json::Value;

use crate::context::ProjectContext;
use crate::write_target::locate;
use crate::write_target::CorpusLayout;

const BLOCK: u8 = 2;
const TOPICS_KEY: &str = "paths.research_topics";

/// Only the fields Claude Code itself writes. `tool_input` stays raw, since
/// the confined agent controls its strings and a value that fails to parse
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
        return unjudged(&call, &Unjudgeable::NoWorkingDirectory);
    };
    let project = Project::at(cwd);
    let confinement = match project.confinement(call.agent_type()) {
        Ok(confinement) => confinement,
        Err(cause) => return unjudged(&call, &cause),
    };
    let Some(agent) = confined_agent(&call, &confinement) else {
        return ExitCode::SUCCESS;
    };
    block_on_panic(&agent);
    #[cfg(feature = "test-loopback")]
    crate::loopback::panic_if_asked();
    judge(&envelope, &project, &agent)
}

fn judge(
    envelope: &Envelope,
    project: &Project,
    agent: &ConfinedAgent,
) -> ExitCode {
    let action = action(envelope, project);
    match decide(&action, agent, &CorpusLayout) {
        Decision::Pass => ExitCode::SUCCESS,
        Decision::Block(block) => {
            let refusal = Refusal {
                agent,
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
/// the lexer judges agent-written text. A hook rather than `catch_unwind`
/// also holds under `panic = "abort"`.
fn block_on_panic(agent: &ConfinedAgent) {
    let reason = InternalFailure(agent).to_string();
    std::panic::set_hook(Box::new(move |_| {
        eprintln!("{reason}");
        std::process::exit(i32::from(BLOCK));
    }));
}

fn unjudged(call: &ToolCall, cause: &Unjudgeable) -> ExitCode {
    let refusal = UnjudgedCall {
        agent_type: call.agent_type(),
        cause,
    };
    eprintln!("{refusal}");
    ExitCode::from(BLOCK)
}

fn pass_with(diagnostic: &str) -> ExitCode {
    eprintln!("accelerator research guard: {diagnostic}; not judging the call");
    ExitCode::SUCCESS
}

/// The project a call runs in, whose configuration is read at most once and
/// only when a judgement needs it.
struct Project {
    cwd: PathBuf,
    context: OnceCell<Result<ProjectContext, String>>,
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

    fn context(&self) -> Result<&ProjectContext, String> {
        self.context
            .get_or_init(|| {
                ProjectContext::enclosing(&self.cwd)
                    .map_err(|error| summary(&error))
            })
            .as_ref()
            .map_err(Clone::clone)
    }

    /// The plugin's own agent names are confined without reading the
    /// configuration; any other name may be a configured one.
    fn confinement(
        &self,
        agent_type: &str,
    ) -> Result<Confinement, Unjudgeable> {
        let confinement = Confinement::default();
        if [DEFAULT_RESEARCHER, DEFAULT_COMPOSER].contains(&agent_type) {
            return Ok(confinement);
        }
        let context = self
            .context()
            .map_err(Unjudgeable::UnreadableConfiguration)?;
        let configured = |key| {
            catalogue::agent_name(context.config.as_ref(), key).map_err(
                |error| Unjudgeable::UnreadableConfiguration(summary(&error)),
            )
        };
        Ok(confinement
            .with_researcher(&configured("researcher")?)
            .with_composer(&configured("composer")?))
    }

    fn topics(&self) -> &Path {
        self.topics.get_or_init(|| {
            let Ok(context) = self.context().inspect_err(|summary| {
                warn_of_defaults(summary);
            }) else {
                return FileConfigStore::discover_root(&self.cwd)
                    .join(default_topics());
            };
            let configured = read_or_default(configured_topics(context))
                .unwrap_or_else(default_topics);
            context.root.join(configured)
        })
    }
}

/// Unreadable configuration never stops the guard confining a research
/// agent it knows by name: it falls back to the catalogue defaults and says
/// so.
fn read_or_default<T>(read: Result<T, ConfigError>) -> Option<T> {
    read.map_err(|error| warn_of_defaults(&summary(&error)))
        .ok()
}

fn warn_of_defaults(summary: &str) {
    eprintln!(
        "accelerator research guard: could not read the configuration, \
         using the defaults: {summary}"
    );
}

fn summary(error: &ConfigError) -> String {
    error
        .to_string()
        .lines()
        .next()
        .unwrap_or_default()
        .to_owned()
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
