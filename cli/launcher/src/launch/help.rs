//! Manifest-driven help augmentation for external subcommands.
//!
//! Clap cannot enumerate them (they are fetched on demand), so each manifest
//! binary is added as a clap subcommand, letting built-ins and sub-binaries
//! render in one native "Commands:" section.

use crate::launch::outbound::resolve::manifest::Manifest;

/// Add each manifest binary to `command` as a child subcommand carrying its
/// description, so clap renders it alongside the built-ins.
///
/// Descriptions are signature-verified but still terminal-rendered, so they are
/// sanitised at this boundary against terminal-escape injection.
///
/// The skip guard is load-bearing: clap `debug_assert`s on a duplicate
/// subcommand, so a manifest binary colliding with a built-in would panic the
/// help render under test and silently double-list in release. A built-in
/// (`version`, `config`, `cache`) is already present on the un-built command and
/// caught by `find_subcommand`; clap's `help` subcommand is not — it is appended
/// only during the build triggered at render time — so the explicit `help` skip
/// is what prevents that collision. A built-in or `help` always shadows a
/// same-named sub-binary, which could therefore never dispatch, so skipping it
/// loses nothing real.
#[must_use]
pub fn augment_with_subbinaries(
    mut command: clap::Command,
    manifest: &Manifest,
) -> clap::Command {
    for (name, entry) in &manifest.binaries {
        let name = sanitize(name);
        if name.is_empty()
            || name == "help"
            || command.find_subcommand(&name).is_some()
        {
            continue;
        }
        command = command.subcommand(
            clap::Command::new(name).about(sanitize(&entry.description)),
        );
    }
    command
}

/// Strip C0/C1 control characters (including the ESC/CSI introducer), operating
/// over Unicode scalars so a multi-byte UTF-8 run is never split.
#[must_use]
pub fn sanitize(text: &str) -> String {
    text.chars().filter(|c| !c.is_control()).collect()
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use clap::CommandFactory as _;

    use crate::launch::inbound::cli::Cli;
    use crate::launch::outbound::resolve::manifest::Manifest;

    use super::{augment_with_subbinaries, sanitize};

    const VERSION: &str = env!("CARGO_PKG_VERSION");

    fn manifest(binaries: &str) -> Result<Manifest, Box<dyn Error>> {
        let json = format!(
            "{{\"schema_version\":1,\"version\":\"{VERSION}\",\
             \"binaries\":{binaries}}}"
        );
        Ok(Manifest::parse_and_validate(json.as_bytes(), VERSION)?)
    }

    #[test]
    fn a_manifest_binary_renders_beside_the_builtins(
    ) -> Result<(), Box<dyn Error>> {
        let manifest = manifest(
            "{\"zzfixture\":{\"description\":\"Fixture tool\",\
             \"platforms\":{}}}",
        )?;
        let mut command = augment_with_subbinaries(Cli::command(), &manifest);
        let help = command.render_help().to_string();
        assert!(help.contains("zzfixture"), "sub-binary missing: {help}");
        assert!(help.contains("Fixture tool"), "description missing: {help}");
        assert!(help.contains("version"), "built-in missing: {help}");
        assert!(
            !help.contains("External subcommands"),
            "the old origin heading survived: {help}"
        );
        Ok(())
    }

    #[test]
    fn colliding_and_empty_names_are_skipped_without_panicking(
    ) -> Result<(), Box<dyn Error>> {
        let manifest = manifest(
            "{\"version\":{\"description\":\"collide\",\"platforms\":{}},\
             \"help\":{\"description\":\"collide\",\"platforms\":{}},\
             \"\":{\"description\":\"empty\",\"platforms\":{}}}",
        )?;
        let mut command = augment_with_subbinaries(Cli::command(), &manifest);
        let versions = command
            .get_subcommands()
            .filter(|sub| sub.get_name() == "version")
            .count();
        assert_eq!(versions, 1, "the built-in version was double-listed");
        let helps = command
            .get_subcommands()
            .filter(|sub| sub.get_name() == "help")
            .count();
        assert_eq!(helps, 0, "a help subcommand was added before the build");
        let empties = command
            .get_subcommands()
            .filter(|sub| sub.get_name().is_empty())
            .count();
        assert_eq!(empties, 0, "an empty-named subcommand was added");
        // The build clap runs at render time `debug_assert`s on a duplicate, so
        // rendering is what proves the `help` collision was actually avoided.
        let _ = command.render_help().to_string();
        Ok(())
    }

    #[test]
    fn sanitize_strips_controls_exactly_and_keeps_utf8() {
        // CSI escape, bell (C0), NEL (C1), tab, and a multi-byte UTF-8 run.
        let dirty = "a\u{1b}[31mb\u{07}\tc\u{0085}—é日";
        assert_eq!(sanitize(dirty), "a[31mbc—é日");
    }
}
