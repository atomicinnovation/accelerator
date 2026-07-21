//! The clap inbound adapter: the top-level `accelerator` command tree.

use std::ffi::OsString;

use clap::{Parser, Subcommand, ValueEnum};
use config_adapters::LegacyPolicy;

/// The `accelerator` command-line surface.
#[derive(Parser)]
#[command(name = "accelerator", disable_version_flag = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Print the version, commit SHA, build date, and target triple.
    Version,
    /// Read or write Accelerator configuration.
    #[command(arg_required_else_help = true)]
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },
    /// Any unknown subcommand and its args, forwarded to the resolved binary.
    #[command(external_subcommand)]
    External(Vec<OsString>),
}

/// Read or write a configuration value.
///
/// Configuration is a dotted `section.key` tree resolved across two levels:
/// `team` is the committed, shared `.accelerator/config.md`, and `personal` is
/// the git-ignored, local `.accelerator/config.local.md` that overrides it.
#[derive(Subcommand)]
pub enum ConfigAction {
    /// Print a configuration value. Without `--level` the value resolves
    /// personal-over-team; with `--level` only that level is read. An unset
    /// key prints the given default, or nothing when none is given.
    Get {
        /// The dotted `section.key` to read (e.g. `agents.reviewer`).
        key: String,
        /// The value to print when the key is unset.
        default: Option<String>,
        /// Read only this level instead of resolving across both.
        #[arg(long)]
        level: Option<Level>,
        /// Suppress the uniform legacy-layout refusal and read the legacy
        /// `.claude/accelerator.md` pair when the current one is absent.
        #[arg(long)]
        allow_legacy_layout: bool,
        /// Never exit non-zero: on a read failure, print nothing and exit 0.
        #[arg(long)]
        fail_safe: bool,
    },
    /// Print a configured `paths.<key>` value. An unset key falls back to the
    /// given default, else the plugin-standard default, else an empty line.
    Path {
        /// The bare path key to read (e.g. `plans`), resolved as `paths.<key>`.
        key: String,
        /// The value to print when the key is unset; wins over the catalogue
        /// default.
        default: Option<String>,
        /// Read only this level instead of resolving across both.
        #[arg(long)]
        level: Option<Level>,
        /// Suppress the uniform legacy-layout refusal and read the legacy
        /// `.claude/accelerator.md` pair when the current one is absent.
        #[arg(long)]
        allow_legacy_layout: bool,
        /// Never exit non-zero: on a read failure, print nothing and exit 0.
        #[arg(long)]
        fail_safe: bool,
    },
    /// Print an agent-name override, falling back to `accelerator:<name>`.
    Agent {
        /// The agent key to read (e.g. `reviewer`), resolved as `agents.<key>`.
        name: String,
        /// Suppress the uniform legacy-layout refusal and read the legacy
        /// `.claude/accelerator.md` pair when the current one is absent.
        #[arg(long)]
        allow_legacy_layout: bool,
        /// Never exit non-zero: on a read failure, print nothing and exit 0.
        #[arg(long)]
        fail_safe: bool,
    },
    /// Print the `## Agent Names` block resolving every agent's configured
    /// override or `accelerator:<name>` default.
    Agents {
        /// Suppress the uniform legacy-layout refusal and read the legacy
        /// `.claude/accelerator.md` pair when the current one is absent.
        #[arg(long)]
        allow_legacy_layout: bool,
        /// Never exit non-zero: on a read failure, render the
        /// `## Agent Names Unavailable` notice and exit 0.
        #[arg(long)]
        fail_safe: bool,
    },
    /// Print a `work.<key>` value with its catalogue default; a
    /// `work.integration` outside the allowed set is a fail-closed refusal.
    Work {
        /// The bare work key to read (e.g. `integration`), resolved as
        /// `work.<key>`.
        key: String,
        /// Suppress the uniform legacy-layout refusal and read the legacy
        /// `.claude/accelerator.md` pair when the current one is absent.
        #[arg(long)]
        allow_legacy_layout: bool,
        /// Never exit non-zero: on a read failure, print nothing and exit 0.
        /// A validation refusal stays fail-closed regardless.
        #[arg(long)]
        fail_safe: bool,
    },
    /// Print the `## Project Context` block from the config-file bodies, and —
    /// with `--skill` — that skill's `## Skill-Specific Context` block after it.
    Context {
        /// Also render this skill's context from
        /// `.accelerator/skills/<name>/context.md`.
        #[arg(long)]
        skill: Option<String>,
        /// Suppress the uniform legacy-layout refusal and read the legacy
        /// `.claude/accelerator.md` pair when the current one is absent.
        #[arg(long)]
        allow_legacy_layout: bool,
        /// Never exit non-zero: on a read failure, render the matching
        /// `## <Name> Unavailable` notice and exit 0.
        #[arg(long)]
        fail_safe: bool,
    },
    /// Print a skill's `## Additional Instructions` block from
    /// `.accelerator/skills/<name>/instructions.md`.
    Instructions {
        /// The skill whose instructions to render, named by its frontmatter
        /// `name`.
        skill: String,
        /// Suppress the uniform legacy-layout refusal and read the legacy
        /// `.claude/accelerator.md` pair when the current one is absent.
        #[arg(long)]
        allow_legacy_layout: bool,
        /// Never exit non-zero: on a read failure, render the
        /// `## Skill Instructions Unavailable` notice and exit 0.
        #[arg(long)]
        fail_safe: bool,
    },
    /// Print the `## Configured Paths` block, or — with `--doc-types` — the 13
    /// doc-type → directory mappings as `type<TAB>dir` lines.
    Paths {
        /// Emit the doc-type → directory mappings instead of the configured
        /// path keys.
        #[arg(long)]
        doc_types: bool,
        /// Include the excluded keys (`tmp`, `templates`, `integrations`) in
        /// the configured-paths block.
        #[arg(long)]
        all: bool,
        /// Rendering; `tsv` for the doc-type mappings, `block` for the
        /// configured paths.
        #[arg(long, value_enum)]
        format: Option<PathsFormat>,
        /// With `--doc-types`, the project root to resolve directories against
        /// (defaults to the current directory).
        root: Option<String>,
        /// Suppress the uniform legacy-layout refusal and read the legacy
        /// `.claude/accelerator.md` pair when the current one is absent.
        #[arg(long)]
        allow_legacy_layout: bool,
        /// Never exit non-zero: on a read failure, render the
        /// `## Configured Paths Unavailable` notice and exit 0. A doc-type
        /// validation refusal stays fail-closed regardless.
        #[arg(long)]
        fail_safe: bool,
    },
    /// Print the `## Effective Configuration` table — every catalogue key with
    /// its effective value and source attribution (team, local, or default).
    Dump {
        /// Suppress the uniform legacy-layout refusal and read the legacy
        /// `.claude/accelerator.md` pair when the current one is absent.
        #[arg(long)]
        allow_legacy_layout: bool,
        /// Never exit non-zero: on a read failure, render the
        /// `## Effective Configuration Unavailable` notice and exit 0.
        #[arg(long)]
        fail_safe: bool,
    },
    /// Print the `## Review Configuration` block and lens catalogue for a
    /// review mode.
    Review {
        /// The review this renders settings for.
        #[arg(value_enum)]
        mode: ReviewMode,
        /// Suppress the uniform legacy-layout refusal and read the legacy
        /// `.claude/accelerator.md` pair when the current one is absent.
        #[arg(long)]
        allow_legacy_layout: bool,
        /// Never exit non-zero: on a read failure, render the
        /// `## Review Configuration Unavailable` notice and exit 0.
        #[arg(long)]
        fail_safe: bool,
    },
    /// Print a brief summary of the active configuration, for the
    /// `SessionStart` hook. `--format hook` wraps it in the `additionalContext`
    /// envelope.
    Summary {
        /// Rendering: `plain` text, or the `SessionStart` `hook` envelope.
        #[arg(long, value_enum)]
        format: Option<SummaryFormat>,
        /// Suppress the uniform legacy-layout refusal and read the legacy
        /// `.claude/accelerator.md` pair when the current one is absent.
        #[arg(long)]
        allow_legacy_layout: bool,
        /// Never exit non-zero: on a read failure, print nothing and exit 0.
        #[arg(long)]
        fail_safe: bool,
    },
}

/// How `config summary` renders its output.
#[derive(Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum SummaryFormat {
    /// Plain summary text.
    Plain,
    /// The `SessionStart` `additionalContext` JSON envelope.
    Hook,
}

/// Which review a `config review` renders settings for.
#[derive(Clone, Copy, ValueEnum)]
pub enum ReviewMode {
    Pr,
    Plan,
    WorkItem,
}

impl From<ReviewMode> for crate::config_command::core::review::Mode {
    fn from(mode: ReviewMode) -> Self {
        match mode {
            ReviewMode::Pr => Self::Pr,
            ReviewMode::Plan => Self::Plan,
            ReviewMode::WorkItem => Self::WorkItem,
        }
    }
}

/// How `config paths` renders its output.
#[derive(Clone, Copy, ValueEnum)]
pub enum PathsFormat {
    /// The `## Configured Paths` markdown block.
    Block,
    /// Tab-separated `type<TAB>dir` lines.
    Tsv,
}

impl ConfigAction {
    /// The legacy policy this action's `--allow-legacy-layout` flag selects.
    /// Only the read subcommands carry the flag; the mutating ones reject it.
    #[must_use]
    pub const fn legacy_policy(&self) -> LegacyPolicy {
        let allow = match self {
            Self::Get {
                allow_legacy_layout,
                ..
            }
            | Self::Path {
                allow_legacy_layout,
                ..
            }
            | Self::Agent {
                allow_legacy_layout,
                ..
            }
            | Self::Agents {
                allow_legacy_layout,
                ..
            }
            | Self::Work {
                allow_legacy_layout,
                ..
            }
            | Self::Context {
                allow_legacy_layout,
                ..
            }
            | Self::Instructions {
                allow_legacy_layout,
                ..
            }
            | Self::Paths {
                allow_legacy_layout,
                ..
            }
            | Self::Dump {
                allow_legacy_layout,
                ..
            }
            | Self::Review {
                allow_legacy_layout,
                ..
            }
            | Self::Summary {
                allow_legacy_layout,
                ..
            } => *allow_legacy_layout,
        };
        if allow {
            LegacyPolicy::Allow
        } else {
            LegacyPolicy::Reject
        }
    }

    /// The directory config resolution should start from: the `--doc-types`
    /// `[root]` positional, else `None` for the current directory. This is how
    /// `config paths --doc-types <root>` resolves against `<root>` rather than
    /// the caller's CWD, matching the bash resolver's `( cd "$root" && … )`.
    #[must_use]
    pub fn resolution_root(&self) -> Option<&str> {
        match self {
            Self::Paths { root, .. } => root.as_deref(),
            _ => None,
        }
    }
}

/// Which configuration level a command reads.
#[derive(Clone, Copy, ValueEnum)]
pub enum Level {
    /// The committed, shared `.accelerator/config.md`.
    Team,
    /// The git-ignored, local `.accelerator/config.local.md` (overrides team).
    Personal,
}

impl From<Level> for config::Level {
    fn from(level: Level) -> Self {
        match level {
            Level::Team => Self::Team,
            Level::Personal => Self::Personal,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error;
    use std::ffi::OsString;

    use clap::Parser as _;

    use super::{Cli, Command};

    #[test]
    fn an_unknown_subcommand_routes_to_external_with_its_args(
    ) -> Result<(), Box<dyn Error>> {
        let cli = Cli::try_parse_from(["accelerator", "frobnicate", "--flag"])?;
        match cli.command {
            Command::External(raw) => assert_eq!(
                raw,
                vec![OsString::from("frobnicate"), OsString::from("--flag")]
            ),
            Command::Version | Command::Config { .. } => {
                return Err("routed away from External".into())
            }
        }
        Ok(())
    }

    #[test]
    fn a_known_subcommand_routes_to_its_builtin() -> Result<(), Box<dyn Error>>
    {
        let cli = Cli::try_parse_from(["accelerator", "version"])?;
        assert!(matches!(cli.command, Command::Version));
        Ok(())
    }

    #[test]
    fn config_get_parses_its_key_and_flags() -> Result<(), Box<dyn Error>> {
        let cli = Cli::try_parse_from([
            "accelerator",
            "config",
            "get",
            "agents.reviewer",
            "--fail-safe",
        ])?;
        match cli.command {
            Command::Config {
                action: super::ConfigAction::Get { key, fail_safe, .. },
            } => {
                assert_eq!(key, "agents.reviewer");
                assert!(fail_safe);
            }
            _ => return Err("did not route to config get".into()),
        }
        Ok(())
    }
}
