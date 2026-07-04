//! The launcher's dispatch/resolution core and the ports it speaks through.
//!
//! Depends on std + `kernel::Error` only; the concrete adapters live under
//! `launch::outbound`. cargo-pup enforces that inward direction.

use std::ffi::OsString;
use std::fmt;
use std::fmt::Display;
use std::fmt::Formatter;
use std::path::Path;
use std::path::PathBuf;

/// A parsed external subcommand: the sub-binary name plus the args to forward.
///
/// Git-style — `accelerator foo a b` resolves the binary named `foo` and
/// forwards `[a, b]` to it; the name is consumed for resolution, not passed on.
/// Both are [`OsString`] so a non-UTF-8 argument survives verbatim to the exec'd
/// child.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExternalCommand {
    pub name: OsString,
    pub args: Vec<OsString>,
}

impl ExternalCommand {
    /// Split clap's raw `External` vector into name + forwarded args.
    ///
    /// # Errors
    ///
    /// [`ResolutionError::EmptyCommand`] if the vector is empty (clap should
    /// never hand us one, but the core refuses it rather than index blindly).
    pub fn from_raw(raw: Vec<OsString>) -> Result<Self, ResolutionError> {
        let mut parts = raw.into_iter();
        let name = parts.next().ok_or(ResolutionError::EmptyCommand)?;
        Ok(Self {
            name,
            args: parts.collect(),
        })
    }
}

/// The launcher's rich, local failure taxonomy — each variant carries the
/// payload its diagnostic needs.
///
/// Maps into the small shared [`kernel::Error`] at the dispatch boundary
/// (`kernel` is the lower crate and cannot name this type), so subdomains never
/// compile against variants they cannot produce. Phase 1 constructs only the
/// dispatch/override/exec variants; the fetch/verify/cache variants are added as
/// the real resolver lands.
#[derive(Debug)]
pub enum ResolutionError {
    /// clap handed dispatch an empty external-subcommand vector.
    EmptyCommand,
    /// A subcommand name cannot derive a valid `ACCELERATOR_<SUB>_BIN` override
    /// variable (empty, a leading digit, or a character that would collide).
    InvalidOverrideName { name: String, detail: String },
    /// The requested sub-binary could not be resolved to a path.
    Unresolved { name: OsString },
    /// `exec` of a resolved binary failed (it only returns on failure).
    Exec {
        program: PathBuf,
        source: std::io::Error,
    },
}

impl Display for ResolutionError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyCommand => {
                write!(formatter, "no external subcommand was given")
            }
            Self::InvalidOverrideName { name, detail } => write!(
                formatter,
                "subcommand '{name}' cannot be used as an override: {detail}"
            ),
            Self::Unresolved { name } => write!(
                formatter,
                "could not resolve subcommand '{}' to a binary",
                name.to_string_lossy()
            ),
            Self::Exec { program, source } => write!(
                formatter,
                "failed to exec {}: {source}",
                program.display()
            ),
        }
    }
}

impl std::error::Error for ResolutionError {}

impl From<ResolutionError> for kernel::Error {
    fn from(error: ResolutionError) -> Self {
        Self::Failed(error.to_string())
    }
}

/// Resolves a sub-binary name to an executable path — a driven/outbound port.
pub trait ResolveBinary {
    /// # Errors
    ///
    /// A [`ResolutionError`] when the binary cannot be produced.
    fn resolve(
        &self,
        command: &ExternalCommand,
    ) -> Result<PathBuf, ResolutionError>;
}

/// Replaces the current process with a resolved binary — a driven/outbound port.
///
/// Modelled as a port so dispatch's resolve→exec wiring is unit-testable with a
/// recording fake; the real Unix `exec` cannot be tested in-process (it would
/// replace the test runner), so its behaviour is proven by black-box tests.
pub trait ExecBinary {
    /// Returns only on failure (a successful `exec` replaces the process), so
    /// the return type is the error that prevented replacement.
    fn exec(&self, program: &Path, args: &[OsString]) -> ResolutionError;
}

/// Resolve the sub-binary and exec it, forwarding its args.
///
/// Only ever returns an error: a successful `exec` replaces this process, so
/// control returns here solely when resolution or exec failed.
pub fn run_external(
    resolver: &impl ResolveBinary,
    executor: &impl ExecBinary,
    command: &ExternalCommand,
) -> ResolutionError {
    match resolver.resolve(command) {
        Ok(program) => executor.exec(&program, &command.args),
        Err(error) => error,
    }
}

/// Derive the `ACCELERATOR_<SUB>_BIN` override variable name for a subcommand.
///
/// The single shared, total normalisation both the Phase 1 and the real
/// resolver call, so the derivation cannot diverge: uppercase the name and map
/// every `-` to `_`, giving `frobnicate-thing` → `ACCELERATOR_FROBNICATE_THING_BIN`.
///
/// The mapping is not injective in general (`frobnicate_thing` would collide with
/// `frobnicate-thing`, and a leading digit is not a valid identifier), so a name
/// is admitted only if it is a valid, collision-free source: it must start with
/// an ASCII letter and contain only ASCII letters, digits, and hyphens. A name
/// containing `_` is refused precisely because it would collide with its
/// hyphenated form; the shipped sub-binary names are a curated, hyphenated,
/// non-colliding set, so this guards a future name choice rather than a live
/// case.
///
/// # Errors
///
/// [`ResolutionError::InvalidOverrideName`] for an empty name, a leading
/// non-letter (including a digit), or a colliding/invalid character.
pub fn derive_override_var(name: &str) -> Result<String, ResolutionError> {
    let mut chars = name.chars();
    let starts_with_letter =
        chars.next().is_some_and(|c| c.is_ascii_alphabetic());
    let rest_is_clean =
        name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-');
    if !starts_with_letter || !rest_is_clean {
        return Err(ResolutionError::InvalidOverrideName {
            name: name.to_owned(),
            detail: "must start with a letter and contain only letters, \
                     digits, and hyphens"
                .to_owned(),
        });
    }
    let mut var = String::with_capacity("ACCELERATOR_".len() + name.len() + 4);
    var.push_str("ACCELERATOR_");
    for c in name.chars() {
        var.push(if c == '-' {
            '_'
        } else {
            c.to_ascii_uppercase()
        });
    }
    var.push_str("_BIN");
    Ok(var)
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::error::Error;
    use std::ffi::OsString;
    use std::path::{Path, PathBuf};

    use super::{
        derive_override_var, run_external, ExecBinary, ExternalCommand,
        ResolutionError, ResolveBinary,
    };

    fn command(name: &str, args: &[&str]) -> ExternalCommand {
        ExternalCommand {
            name: OsString::from(name),
            args: args.iter().map(OsString::from).collect(),
        }
    }

    struct FixedResolver {
        path: PathBuf,
    }

    impl ResolveBinary for FixedResolver {
        fn resolve(
            &self,
            _command: &ExternalCommand,
        ) -> Result<PathBuf, ResolutionError> {
            Ok(self.path.clone())
        }
    }

    struct FailingResolver;

    impl ResolveBinary for FailingResolver {
        fn resolve(
            &self,
            command: &ExternalCommand,
        ) -> Result<PathBuf, ResolutionError> {
            Err(ResolutionError::Unresolved {
                name: command.name.clone(),
            })
        }
    }

    #[derive(Default)]
    struct RecordingExec {
        seen: RefCell<Option<(PathBuf, Vec<OsString>)>>,
    }

    impl ExecBinary for RecordingExec {
        fn exec(&self, program: &Path, args: &[OsString]) -> ResolutionError {
            *self.seen.borrow_mut() =
                Some((program.to_path_buf(), args.to_vec()));
            // A real exec never returns on success; the fake reports "attempted"
            // so the plumbing is observable without replacing the test runner.
            ResolutionError::Exec {
                program: program.to_path_buf(),
                source: std::io::Error::other("fake exec"),
            }
        }
    }

    #[test]
    fn from_raw_splits_name_from_forwarded_args() -> Result<(), Box<dyn Error>>
    {
        let parsed = ExternalCommand::from_raw(vec![
            OsString::from("foo"),
            OsString::from("--flag"),
            OsString::from("value"),
        ])?;
        assert_eq!(parsed.name, OsString::from("foo"));
        assert_eq!(
            parsed.args,
            vec![OsString::from("--flag"), OsString::from("value")]
        );
        Ok(())
    }

    #[test]
    fn from_raw_rejects_an_empty_vector() {
        assert!(matches!(
            ExternalCommand::from_raw(vec![]),
            Err(ResolutionError::EmptyCommand)
        ));
    }

    #[test]
    fn run_external_execs_the_resolved_path_with_forwarded_args(
    ) -> Result<(), Box<dyn Error>> {
        let resolver = FixedResolver {
            path: PathBuf::from("/cache/accelerator-foo"),
        };
        let executor = RecordingExec::default();
        let _ = run_external(
            &resolver,
            &executor,
            &command("foo", ["a", "b"].as_slice()),
        );
        let seen = executor.seen.borrow();
        let (program, args) = seen.as_ref().ok_or("exec was not attempted")?;
        assert_eq!(program, &PathBuf::from("/cache/accelerator-foo"));
        assert_eq!(args, &vec![OsString::from("a"), OsString::from("b")]);
        Ok(())
    }

    #[test]
    fn run_external_returns_the_resolve_error_without_exec() {
        let executor = RecordingExec::default();
        let error =
            run_external(&FailingResolver, &executor, &command("foo", &[]));
        assert!(matches!(error, ResolutionError::Unresolved { .. }));
        assert!(executor.seen.borrow().is_none(), "exec must not run");
    }

    #[test]
    fn resolution_error_maps_into_a_kernel_failed_diagnostic() {
        let error = ResolutionError::Unresolved {
            name: OsString::from("frobnicate"),
        };
        let kernel_error: kernel::Error = error.into();
        assert!(kernel_error.to_string().contains("frobnicate"));
    }

    #[test]
    fn derive_override_var_uppercases_and_maps_hyphens(
    ) -> Result<(), Box<dyn Error>> {
        assert_eq!(
            derive_override_var("frobnicate")?,
            "ACCELERATOR_FROBNICATE_BIN"
        );
        assert_eq!(
            derive_override_var("frobnicate-thing")?,
            "ACCELERATOR_FROBNICATE_THING_BIN"
        );
        Ok(())
    }

    #[test]
    fn derive_override_var_rejects_a_colliding_underscore_name() {
        // `frobnicate_thing` would collide with `frobnicate-thing`; refuse it.
        assert!(matches!(
            derive_override_var("frobnicate_thing"),
            Err(ResolutionError::InvalidOverrideName { .. })
        ));
    }

    #[test]
    fn derive_override_var_rejects_a_leading_digit() {
        assert!(matches!(
            derive_override_var("9lives"),
            Err(ResolutionError::InvalidOverrideName { .. })
        ));
    }

    #[test]
    fn derive_override_var_rejects_an_empty_name() {
        assert!(matches!(
            derive_override_var(""),
            Err(ResolutionError::InvalidOverrideName { .. })
        ));
    }
}
