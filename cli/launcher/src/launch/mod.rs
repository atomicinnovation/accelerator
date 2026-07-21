//! The dispatch boundary: routes the parsed command tree to a built-in handler
//! or to external resolution + exec.

pub mod core;
pub mod help;
pub mod inbound;
pub mod outbound;

use config::ConfigError;

use crate::config_command::core::{ConfigStack, OnFailure};
use crate::config_command::inbound::cli as config_cli;
use crate::launch::core::{
    run_external, ExecBinary, ExternalCommand, ResolveBinary,
};
use crate::launch::inbound::cli::{Cli, Command, ConfigAction};
use crate::version::core::ReportVersion;
use crate::version::inbound::cli as version_cli;

const fn on_failure(fail_safe: bool) -> OnFailure {
    if fail_safe {
        OnFailure::Degrade
    } else {
        OnFailure::Fail
    }
}

/// Maps the launcher's clap `ConfigAction` onto the hexagon's own request type,
/// so `config_command` never names the launcher's clap tree.
fn to_action(action: &ConfigAction) -> config_cli::Action {
    match action {
        ConfigAction::Get {
            key,
            default,
            level,
            fail_safe,
            ..
        } => config_cli::Action::Get {
            key: key.clone(),
            default: default.clone(),
            level: level.map(Into::into),
            on_failure: on_failure(*fail_safe),
        },
        ConfigAction::Path {
            key,
            default,
            level,
            fail_safe,
            ..
        } => config_cli::Action::Path {
            key: key.clone(),
            default: default.clone(),
            level: level.map(Into::into),
            on_failure: on_failure(*fail_safe),
        },
        ConfigAction::Agent {
            name, fail_safe, ..
        } => config_cli::Action::Agent {
            name: name.clone(),
            on_failure: on_failure(*fail_safe),
        },
        ConfigAction::Agents { fail_safe, .. } => config_cli::Action::Agents {
            on_failure: on_failure(*fail_safe),
        },
        ConfigAction::Work { key, fail_safe, .. } => config_cli::Action::Work {
            key: key.clone(),
            on_failure: on_failure(*fail_safe),
        },
        ConfigAction::Context {
            skill, fail_safe, ..
        } => config_cli::Action::Context {
            skill: skill.clone(),
            on_failure: on_failure(*fail_safe),
        },
        ConfigAction::Instructions {
            skill, fail_safe, ..
        } => config_cli::Action::Instructions {
            skill: skill.clone(),
            on_failure: on_failure(*fail_safe),
        },
        ConfigAction::Paths {
            doc_types,
            all,
            fail_safe,
            ..
        } => config_cli::Action::Paths {
            doc_types: *doc_types,
            all: *all,
            on_failure: on_failure(*fail_safe),
        },
    }
}

/// Route the parsed command: built-ins run in-process, an external subcommand
/// resolves and execs (replacing this process on success).
///
/// `compose_config` builds the `config` port bundle lazily — invoked only when
/// the `Config` arm routes to it, so `version` and external subcommands never
/// pay root discovery or the legacy guard. It is opaque here, so this module
/// names no concrete adapter.
///
/// # Errors
///
/// A [`kernel::Error`] when a built-in fails or an external subcommand cannot
/// be resolved or exec'd.
pub fn dispatch(
    cli: &Cli,
    reporter: &impl ReportVersion,
    resolver: &impl ResolveBinary,
    executor: &impl ExecBinary,
    compose_config: impl FnOnce() -> Result<ConfigStack, ConfigError>,
) -> Result<(), kernel::Error> {
    match &cli.command {
        Command::Version => {
            version_cli::report(reporter);
            Ok(())
        }
        Command::Config { action } => {
            let stack = compose_config()?;
            Ok(config_cli::run(&stack, &to_action(action))?)
        }
        Command::External(raw) => {
            let command = ExternalCommand::from_raw(raw.clone())?;
            // A successful exec never returns, so reaching here is always error.
            Err(run_external(resolver, executor, &command).into())
        }
    }
}
