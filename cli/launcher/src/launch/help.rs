//! Manifest-driven help synthesis for external subcommands (which clap cannot
//! enumerate, as they are fetched on demand).

use std::fmt::Write as _;

use crate::launch::outbound::resolve::manifest::Manifest;

/// Build the "External subcommands" help section from the manifest, or `None`
/// when it lists no binaries.
///
/// Descriptions are signature-verified but still terminal-rendered, so they are
/// sanitised at this boundary against terminal-escape injection.
#[must_use]
pub fn external_subcommands_section(manifest: &Manifest) -> Option<String> {
    if manifest.binaries.is_empty() {
        return None;
    }
    let width = manifest
        .binaries
        .keys()
        .map(|name| sanitize(name).len())
        .max()
        .unwrap_or(0);
    let mut section = String::from("External subcommands:");
    for (name, entry) in &manifest.binaries {
        let name = sanitize(name);
        let description = sanitize(&entry.description);
        let _ = write!(section, "\n  {name:<width$}  {description}");
    }
    Some(section)
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

    use crate::launch::outbound::resolve::manifest::Manifest;

    use super::{external_subcommands_section, sanitize};

    const VERSION: &str = env!("CARGO_PKG_VERSION");

    fn manifest(binaries: &str) -> Result<Manifest, Box<dyn Error>> {
        let json = format!(
            "{{\"schema_version\":1,\"version\":\"{VERSION}\",\
             \"binaries\":{binaries}}}"
        );
        Ok(Manifest::parse_and_validate(json.as_bytes(), VERSION)?)
    }

    #[test]
    fn renders_a_line_matching_the_name_and_description(
    ) -> Result<(), Box<dyn Error>> {
        let manifest = manifest(
            "{\"foo\":{\"description\":\"Bar tool\",\"platforms\":{}}}",
        )?;
        let section = external_subcommands_section(&manifest)
            .ok_or("expected section")?;
        assert!(section.contains("foo"));
        assert!(section.contains("Bar tool"));
        Ok(())
    }

    #[test]
    fn no_binaries_yields_no_section() -> Result<(), Box<dyn Error>> {
        assert!(external_subcommands_section(&manifest("{}")?).is_none());
        Ok(())
    }

    #[test]
    fn sanitize_strips_controls_exactly_and_keeps_utf8() {
        // CSI escape, bell (C0), NEL (C1), tab, and a multi-byte UTF-8 run.
        let dirty = "a\u{1b}[31mb\u{07}\tc\u{0085}—é日";
        assert_eq!(sanitize(dirty), "a[31mbc—é日");
    }
}
