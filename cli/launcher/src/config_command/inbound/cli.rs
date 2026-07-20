//! Maps a parsed `config` request onto the injected core and presents the
//! result.
//!
//! The scalar reads (`get`, `path`, `agent`) emit exactly the resolved value
//! plus one newline on success. Under [`OnFailure::Degrade`] a read failure is
//! absorbed — nothing on stdout, the diagnostic on stderr, exit zero — so a
//! caller splicing this stdout into a prompt is never handed a non-zero exit.

use config::{catalogue, ConfigError, Key, Level, Resolved};

use crate::config_command::core::{ConfigStack, OnFailure};

/// A parsed `config` request, owned by this module so the hexagon never names
/// the launcher's clap tree. The composition boundary maps the clap
/// `ConfigAction` onto this.
pub enum Action {
    Get {
        key: String,
        default: Option<String>,
        level: Option<Level>,
        on_failure: OnFailure,
    },
    Path {
        key: String,
        default: Option<String>,
        level: Option<Level>,
        on_failure: OnFailure,
    },
    Agent {
        name: String,
        on_failure: OnFailure,
    },
}

/// Runs a parsed `config` request against the composed stack.
///
/// # Errors
///
/// A [`ConfigError`] when a read fails and the request's `on_failure` is
/// [`OnFailure::Fail`]; under [`OnFailure::Degrade`] the same failure is
/// absorbed and this succeeds.
pub fn run(stack: &ConfigStack, action: &Action) -> Result<(), ConfigError> {
    match action {
        Action::Get {
            key,
            default,
            level,
            on_failure,
        } => emit(
            resolve_get(stack, key, default.as_deref(), *level),
            *on_failure,
        ),
        Action::Path {
            key,
            default,
            level,
            on_failure,
        } => emit(
            resolve_path(stack, key, default.as_deref(), *level),
            *on_failure,
        ),
        Action::Agent { name, on_failure } => {
            emit(resolve_agent(stack, name), *on_failure)
        }
    }
}

fn resolve_get(
    stack: &ConfigStack,
    raw_key: &str,
    default: Option<&str>,
    level: Option<Level>,
) -> Result<Scalar, ConfigError> {
    let key = Key::parse(raw_key)?;
    let value = match stack.config().get(&key, level)? {
        Resolved::Found(value) => config::render_value(&value),
        Resolved::Absent => default.unwrap_or_default().to_owned(),
    };
    Ok(Scalar::bare(value))
}

fn resolve_path(
    stack: &ConfigStack,
    raw_key: &str,
    default: Option<&str>,
    level: Option<Level>,
) -> Result<Scalar, ConfigError> {
    let full = format!("paths.{raw_key}");
    let key = Key::parse(&full)?;
    let mut warnings = Vec::new();
    let fallback = path_fallback(default, raw_key, &full, &mut warnings);
    let value = match stack.config().get(&key, level)? {
        Resolved::Found(value) => config::render_value(&value),
        Resolved::Absent => fallback,
    };
    Ok(Scalar { value, warnings })
}

/// The value a `path` miss falls back to: an explicit non-empty default wins,
/// else the catalogue default, else empty with a stderr warning naming the key.
fn path_fallback(
    default: Option<&str>,
    raw_key: &str,
    full_key: &str,
    warnings: &mut Vec<String>,
) -> String {
    if let Some(explicit) = default.filter(|value| !value.is_empty()) {
        return explicit.to_owned();
    }
    if let Some(value) = catalogue::default_for(full_key) {
        return config::render_value(&value);
    }
    warnings.push(unknown_path_key_warning(raw_key));
    String::new()
}

fn resolve_agent(
    stack: &ConfigStack,
    name: &str,
) -> Result<Scalar, ConfigError> {
    let key = Key::parse(&format!("agents.{name}"))?;
    let value = match stack.config().get(&key, None)? {
        Resolved::Found(value) => config::render_value(&value),
        Resolved::Absent => format!("{}{name}", catalogue::AGENT_PREFIX),
    };
    Ok(Scalar::bare(value))
}

fn unknown_path_key_warning(key: &str) -> String {
    match key {
        "design_inventories" | "design_gaps" => format!(
            "accelerator config path: key '{key}' was renamed by migration \
             0004 to 'research_{key}'; run /accelerator:migrate"
        ),
        _ => format!(
            "accelerator config path: unknown key '{key}' — no centralized \
             default"
        ),
    }
}

/// A resolved scalar and any stderr warnings the resolution accrued.
struct Scalar {
    value: String,
    warnings: Vec<String>,
}

impl Scalar {
    const fn bare(value: String) -> Self {
        Self {
            value,
            warnings: Vec::new(),
        }
    }
}

fn emit(
    resolved: Result<Scalar, ConfigError>,
    on_failure: OnFailure,
) -> Result<(), ConfigError> {
    match resolved {
        Ok(scalar) => {
            for warning in &scalar.warnings {
                eprintln!("{warning}");
            }
            println!("{}", scalar.value);
            Ok(())
        }
        Err(error) if on_failure == OnFailure::Degrade => {
            eprintln!("{error}");
            Ok(())
        }
        Err(error) => Err(error),
    }
}
