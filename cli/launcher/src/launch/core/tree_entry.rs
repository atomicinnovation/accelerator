//! The trust boundary between an untrusted archive and the filesystem.
//!
//! An allowlist, not a denylist: regular files and directories are admitted,
//! symlinks only when their target stays inside the tree, and everything else
//! is refused. A refusal fails the whole materialisation rather than skipping
//! the entry, so an archive cannot quietly deliver a partial tree.
//!
//! The rules live here, in the domain, rather than in the adapter that reads
//! the archive: as a pure function over a described entry the whole rejection
//! matrix is a table-driven test, where in the adapter every case would have to
//! be a hand-built tarball.

use std::collections::BTreeSet;
use std::ffi::OsStr;

/// The longest single path component admitted, matching the limit every
/// filesystem the launcher targets enforces anyway.
const MAX_COMPONENT: usize = 255;
/// The longest whole path admitted.
const MAX_PATH: usize = 4096;

/// What the archive says an entry is.
///
/// Everything an archive can carry that is not a regular file, a directory or a
/// symlink is named rather than collapsed, so a refusal can say which shape it
/// refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryKind {
    File,
    Directory,
    Symlink,
    Hardlink,
    Fifo,
    Device,
    Socket,
    /// A PAX or GNU record extending the header that follows it.
    LongNameOverride,
    Unknown,
}

/// One archive entry, as the adapter reads it from the header.
///
/// `path` and `link_target` are `OsStr` rather than `str` so the charset policy
/// is decided here rather than by an adapter that had to make the name a
/// `String` before it could pass it on.
pub struct DescribedEntry<'a> {
    pub kind: EntryKind,
    pub path: &'a OsStr,
    pub mode: u32,
    pub size: u64,
    pub link_target: Option<&'a OsStr>,
}

/// The ceilings extraction runs under, read from the signed attestation.
pub struct ExtractionLimits {
    pub uncompressed_size: u64,
    pub entry_count: u64,
}

/// What extraction has admitted so far, so a decompression bomb aborts partway
/// rather than after filling the disk.
#[derive(Debug, Default, Clone, Copy)]
pub struct ExtractionProgress {
    pub bytes: u64,
    pub entries: u64,
}

/// An admitted entry, with the mode the extractor must actually apply.
#[derive(Debug, PartialEq, Eq)]
pub struct AdmittedEntry {
    pub kind: EntryKind,
    /// Masked to `0755`/`0644`, so setuid, setgid and sticky bits cannot
    /// survive extraction whatever the archive asked for.
    pub mode: u32,
}

/// Why an entry was refused.
#[derive(Debug, PartialEq, Eq)]
pub enum EntryRefusal {
    EmptyPath,
    NonUtf8Name,
    ControlCharacter,
    AbsolutePath,
    ParentComponent,
    DriveOrUncPrefix,
    ComponentTooLong { limit: usize },
    PathTooLong { limit: usize },
    UnsupportedKind(EntryKind),
    MissingLinkTarget,
    NonUtf8LinkTarget,
    EscapingLinkTarget,
    DuplicatePath,
    SizeCeilingExceeded { limit: u64 },
    EntryCeilingExceeded { limit: u64 },
}

/// Decide whether `entry` may be written, and with what mode.
///
/// `written` is the set of paths already admitted, so a second entry for a path
/// is refused rather than silently overwriting the first — the last-one-wins
/// behaviour much of the tar CVE history turns on.
///
/// # Errors
///
/// [`EntryRefusal`] naming the rule the entry broke.
pub fn classify_entry(
    entry: &DescribedEntry<'_>,
    limits: &ExtractionLimits,
    progress: &ExtractionProgress,
    written: &BTreeSet<String>,
) -> Result<AdmittedEntry, EntryRefusal> {
    let path = admissible_path(entry.path)?;

    if progress.entries >= limits.entry_count {
        return Err(EntryRefusal::EntryCeilingExceeded {
            limit: limits.entry_count,
        });
    }
    if progress.bytes.saturating_add(entry.size) > limits.uncompressed_size {
        return Err(EntryRefusal::SizeCeilingExceeded {
            limit: limits.uncompressed_size,
        });
    }
    if written.contains(path) {
        return Err(EntryRefusal::DuplicatePath);
    }

    match entry.kind {
        EntryKind::File | EntryKind::Directory => {}
        EntryKind::Symlink => admissible_link_target(entry, path)?,
        kind => return Err(EntryRefusal::UnsupportedKind(kind)),
    }

    Ok(AdmittedEntry {
        kind: entry.kind,
        mode: masked_mode(entry.kind, entry.mode),
    })
}

/// The seal is a deterministic function of the executable bit, so the extractor
/// and `verify` compute the same expected mode from the same recorded one.
#[must_use]
pub const fn masked_mode(kind: EntryKind, mode: u32) -> u32 {
    match kind {
        EntryKind::Directory => 0o755,
        _ if mode & 0o111 != 0 => 0o755,
        _ => 0o644,
    }
}

fn admissible_path(path: &OsStr) -> Result<&str, EntryRefusal> {
    let path = path.to_str().ok_or(EntryRefusal::NonUtf8Name)?;
    if path.is_empty() {
        return Err(EntryRefusal::EmptyPath);
    }
    if path.len() > MAX_PATH {
        return Err(EntryRefusal::PathTooLong { limit: MAX_PATH });
    }
    if path.chars().any(char::is_control) {
        return Err(EntryRefusal::ControlCharacter);
    }
    // A drive letter or UNC prefix is a rooted path on the platform that
    // understands it, and a plausible-looking relative one everywhere else.
    if path.starts_with("\\\\") || has_drive_prefix(path) {
        return Err(EntryRefusal::DriveOrUncPrefix);
    }
    if path.starts_with('/') {
        return Err(EntryRefusal::AbsolutePath);
    }
    for component in path.split('/') {
        if component == ".." {
            return Err(EntryRefusal::ParentComponent);
        }
        if component.len() > MAX_COMPONENT {
            return Err(EntryRefusal::ComponentTooLong {
                limit: MAX_COMPONENT,
            });
        }
    }
    Ok(path)
}

fn has_drive_prefix(path: &str) -> bool {
    let mut chars = path.chars();
    chars.next().is_some_and(|c| c.is_ascii_alphabetic())
        && chars.next() == Some(':')
}

/// A lexical containment check on the link target.
///
/// This is the first of two gates and not the load-bearing one: extraction
/// resolves each component under a directory fd that refuses to traverse a
/// symlink, which is what closes the symlink-then-traverse and TOCTOU cases a
/// lexical check cannot see. Refusing the obvious escapes here keeps them out
/// of the adapter entirely.
fn admissible_link_target(
    entry: &DescribedEntry<'_>,
    path: &str,
) -> Result<(), EntryRefusal> {
    let target = entry
        .link_target
        .ok_or(EntryRefusal::MissingLinkTarget)?
        .to_str()
        .ok_or(EntryRefusal::NonUtf8LinkTarget)?;
    if target.is_empty() {
        return Err(EntryRefusal::MissingLinkTarget);
    }
    if target.starts_with('/') || has_drive_prefix(target) {
        return Err(EntryRefusal::EscapingLinkTarget);
    }

    // Walk the target from the link's own directory and refuse any prefix that
    // steps above the tree root.
    let mut depth = path.split('/').count() - 1;
    for component in target.split('/') {
        match component {
            "" | "." => {}
            ".." => {
                depth = depth
                    .checked_sub(1)
                    .ok_or(EntryRefusal::EscapingLinkTarget)?;
            }
            _ => depth += 1,
        }
    }
    Ok(())
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use std::collections::BTreeSet;
    use std::ffi::OsStr;

    use super::{
        classify_entry, AdmittedEntry, DescribedEntry, EntryKind, EntryRefusal,
        ExtractionLimits, ExtractionProgress,
    };

    fn entry(path: &str, kind: EntryKind) -> DescribedEntry<'_> {
        DescribedEntry {
            kind,
            path: OsStr::new(path),
            mode: 0o644,
            size: 0,
            link_target: None,
        }
    }

    fn limits() -> ExtractionLimits {
        ExtractionLimits {
            uncompressed_size: 1 << 20,
            entry_count: 100,
        }
    }

    fn nothing_written() -> BTreeSet<String> {
        BTreeSet::new()
    }

    fn classify(
        entry: &DescribedEntry<'_>,
    ) -> Result<AdmittedEntry, EntryRefusal> {
        classify_entry(
            entry,
            &limits(),
            &ExtractionProgress::default(),
            &nothing_written(),
        )
    }

    fn link<'a>(path: &'a str, target: &'a str) -> DescribedEntry<'a> {
        DescribedEntry {
            kind: EntryKind::Symlink,
            path: OsStr::new(path),
            mode: 0o777,
            size: 0,
            link_target: Some(OsStr::new(target)),
        }
    }

    #[test]
    fn a_parent_traversal_entry_is_rejected() {
        let refusal = classify(&entry("../escape", EntryKind::File))
            .expect_err("a parent-traversal entry must be refused");
        assert_eq!(refusal, EntryRefusal::ParentComponent);
    }

    #[test]
    fn the_name_policy_refuses_each_hostile_shape() {
        let long_component = "a".repeat(256);
        let long_path = vec!["dir"; 2000].join("/");
        let cases: Vec<(String, EntryRefusal)> = vec![
            (String::new(), EntryRefusal::EmptyPath),
            ("/etc/passwd".to_owned(), EntryRefusal::AbsolutePath),
            ("lib/../../escape".to_owned(), EntryRefusal::ParentComponent),
            ("..".to_owned(), EntryRefusal::ParentComponent),
            (
                "C:/windows/system32".to_owned(),
                EntryRefusal::DriveOrUncPrefix,
            ),
            (
                "\\\\server\\share".to_owned(),
                EntryRefusal::DriveOrUncPrefix,
            ),
            ("lib/ev\u{7}il".to_owned(), EntryRefusal::ControlCharacter),
            ("lib/new\nline".to_owned(), EntryRefusal::ControlCharacter),
            (
                long_component,
                EntryRefusal::ComponentTooLong { limit: 255 },
            ),
            (long_path, EntryRefusal::PathTooLong { limit: 4096 }),
        ];
        for (path, expected) in cases {
            assert_eq!(
                classify(&entry(&path, EntryKind::File)),
                Err(expected),
                "path {path:?} was not refused as expected"
            );
        }
    }

    #[cfg(unix)]
    #[test]
    fn a_non_utf8_name_is_refused_rather_than_lossily_converted() {
        use std::os::unix::ffi::OsStrExt as _;

        let raw = OsStr::from_bytes(b"lib/\xff\xfe");
        let refused = classify(&DescribedEntry {
            kind: EntryKind::File,
            path: raw,
            mode: 0o644,
            size: 0,
            link_target: None,
        });
        assert_eq!(refused, Err(EntryRefusal::NonUtf8Name));
    }

    #[test]
    fn only_files_directories_and_contained_symlinks_are_admitted() {
        for kind in [
            EntryKind::Hardlink,
            EntryKind::Fifo,
            EntryKind::Device,
            EntryKind::Socket,
            EntryKind::LongNameOverride,
            EntryKind::Unknown,
        ] {
            assert_eq!(
                classify(&entry("lib/thing", kind)),
                Err(EntryRefusal::UnsupportedKind(kind)),
                "{kind:?} must be refused"
            );
        }
        assert!(classify(&entry("lib/thing", EntryKind::File)).is_ok());
        assert!(classify(&entry("lib", EntryKind::Directory)).is_ok());
    }

    #[test]
    fn a_symlink_is_admitted_only_while_its_target_stays_inside() {
        assert!(classify(&link("lib/here", "sibling")).is_ok());
        assert!(classify(&link("lib/nested/here", "../sibling")).is_ok());
        assert_eq!(
            classify(&link("lib/here", "../../escape")),
            Err(EntryRefusal::EscapingLinkTarget)
        );
        assert_eq!(
            classify(&link("here", "../escape")),
            Err(EntryRefusal::EscapingLinkTarget)
        );
        assert_eq!(
            classify(&link("lib/here", "/etc/passwd")),
            Err(EntryRefusal::EscapingLinkTarget)
        );
        assert_eq!(
            classify(&link("lib/here", "")),
            Err(EntryRefusal::MissingLinkTarget)
        );
    }

    #[test]
    fn a_symlink_climbing_and_returning_within_the_tree_is_admitted() {
        // The lexical rule is about the net position, not about `..` appearing
        // — a link that steps up and back down stays inside.
        assert!(classify(&link("lib/nested/here", "../other/target")).is_ok());
    }

    #[test]
    fn the_ceilings_stop_a_decompression_bomb_partway() {
        let limits = ExtractionLimits {
            uncompressed_size: 1000,
            entry_count: 2,
        };
        let big = DescribedEntry {
            kind: EntryKind::File,
            path: OsStr::new("lib/big"),
            mode: 0o644,
            size: 600,
            link_target: None,
        };
        assert_eq!(
            classify_entry(
                &big,
                &limits,
                &ExtractionProgress {
                    bytes: 500,
                    entries: 1
                },
                &nothing_written()
            ),
            Err(EntryRefusal::SizeCeilingExceeded { limit: 1000 })
        );
        assert_eq!(
            classify_entry(
                &entry("lib/one-too-many", EntryKind::File),
                &limits,
                &ExtractionProgress {
                    bytes: 0,
                    entries: 2
                },
                &nothing_written()
            ),
            Err(EntryRefusal::EntryCeilingExceeded { limit: 2 })
        );
    }

    #[test]
    fn a_second_entry_for_an_already_written_path_is_refused() {
        let mut written = BTreeSet::new();
        written.insert("lib/thing".to_owned());
        assert_eq!(
            classify_entry(
                &entry("lib/thing", EntryKind::File),
                &limits(),
                &ExtractionProgress::default(),
                &written
            ),
            Err(EntryRefusal::DuplicatePath)
        );
    }

    #[test]
    fn setuid_setgid_and_sticky_bits_cannot_survive_the_mask() {
        for mode in [0o4755, 0o2755, 0o1755, 0o6777] {
            let admitted = classify(&DescribedEntry {
                kind: EntryKind::File,
                path: OsStr::new("bin/node"),
                mode,
                size: 0,
                link_target: None,
            })
            .expect("an executable file is admitted");
            assert_eq!(
                admitted.mode, 0o755,
                "{mode:o} kept a bit beyond the executable one"
            );
        }
    }

    #[test]
    fn the_mask_keeps_the_executable_bit_and_nothing_else() {
        let executable = classify(&DescribedEntry {
            kind: EntryKind::File,
            path: OsStr::new("bin/node"),
            mode: 0o700,
            size: 0,
            link_target: None,
        })
        .expect("admitted");
        assert_eq!(executable.mode, 0o755);

        let plain = classify(&entry("lib/data.pak", EntryKind::File))
            .expect("admitted");
        assert_eq!(plain.mode, 0o644);

        let directory =
            classify(&entry("lib", EntryKind::Directory)).expect("admitted");
        assert_eq!(directory.mode, 0o755);
    }
}
