//! Extraction: the untrusted archive becomes a directory tree, or nothing.
//!
//! Containment is `openat` with `O_NOFOLLOW` per path component under a
//! directory fd for the temp root — not a lexical prefix check and not a
//! `canonicalize` after the fact. A lexical check is defeated by a
//! symlink-then-traverse chain, and a check-then-create pair is a TOCTOU
//! window; resolving each component through a fd chain that refuses to traverse
//! a symlink closes both. The pure admission rules run first
//! ([`crate::launch::core::tree_entry`]); this module is what "resolved against
//! the real root as it is created" means in practice.

#[cfg(unix)]
pub use unix::extract_archive;

#[cfg(unix)]
mod unix {
    use std::io::Read;
    use std::os::fd::{AsFd, OwnedFd};
    use std::path::Path;

    use rustix::fs::{Mode, OFlags};
    use sha2::{Digest as _, Sha256};

    use crate::launch::core::tree::TreeError;
    use crate::launch::core::tree_entry::{
        classify_entry, DescribedEntry, EntryKind, ExtractionLimits,
        ExtractionProgress,
    };

    use super::super::table::{FileTable, TableRow, TABLE_NAME};

    /// The size of one read from the archive body.
    const CHUNK: usize = 64 * 1024;

    /// What a completed extraction leaves for the sealer and the pointer.
    #[derive(Debug)]
    pub struct Extracted {
        pub table: FileTable,
        pub uncompressed_bytes: u64,
        pub entry_count: u64,
    }

    /// Extract `reader`'s tar stream into `dest`, verifying every member
    /// against the `.files` table the archive carries as its first entry.
    ///
    /// Each regular file's sha256 is computed as it is written and checked
    /// against its table row, so verification costs no second pass and a
    /// tampered member is caught during extraction rather than afterwards.
    ///
    /// # Errors
    ///
    /// [`TreeError::TableMissing`] if the first member is not the table,
    /// [`TreeError::PathEscape`] for a containment breach, or
    /// [`TreeError::Extraction`] for a rejected entry, a member disagreeing
    /// with the table, or an I/O failure.
    pub fn extract_archive<R: Read>(
        reader: R,
        dest: &Path,
        limits: &ExtractionLimits,
    ) -> Result<Extracted, TreeError> {
        let root = open_dir(dest).map_err(|error| {
            extraction(&format!("cannot open the extraction root: {error}"))
        })?;
        let mut archive = tar::Archive::new(reader);
        let mut entries = archive.entries().map_err(|error| {
            extraction(&format!("unreadable archive: {error}"))
        })?;

        let table = read_table_member(&mut entries)?;
        let mut progress = ExtractionProgress::default();
        let mut written = std::collections::BTreeSet::new();

        for entry in entries {
            let mut entry = entry.map_err(|error| {
                extraction(&format!("unreadable archive entry: {error}"))
            })?;
            let header = entry.header();
            let kind = classify_kind(header.entry_type());
            let mode = header.mode().unwrap_or(0);
            let size = header.size().unwrap_or(0);
            let path_os = owned_path(&entry);
            let link_os = entry.link_name_bytes().map(os_str_from_bytes);

            let admitted = classify_entry(
                &DescribedEntry {
                    kind,
                    path: path_os.as_os_str(),
                    mode,
                    size,
                    link_target: link_os.as_deref(),
                },
                limits,
                &progress,
                &written,
            )
            .map_err(refusal_to_tree_error)?;

            let path = path_os
                .into_string()
                .map_err(|_| extraction("a UTF-8 path became non-UTF-8"))?;
            let row = table.row(&path).ok_or_else(|| {
                extraction(&format!("'{path}' is absent from the file table"))
            })?;

            match admitted.kind {
                EntryKind::Directory => {
                    make_dir(&root, &path, admitted.mode)?;
                }
                EntryKind::File => {
                    let bytes = write_file(
                        &root,
                        &path,
                        admitted.mode,
                        &mut entry,
                        row,
                    )?;
                    progress.bytes = progress.bytes.saturating_add(bytes);
                }
                EntryKind::Symlink => {
                    make_symlink(&root, &path, row)?;
                }
                other => {
                    return Err(extraction(&format!(
                        "{other:?} reached extraction after admission"
                    )))
                }
            }
            progress.entries += 1;
            written.insert(path);
        }

        Ok(Extracted {
            table,
            uncompressed_bytes: progress.bytes,
            entry_count: progress.entries,
        })
    }

    /// The first member must be the table, so single-pass verification cannot
    /// silently degrade into a second inflate to find it.
    fn read_table_member<R: Read>(
        entries: &mut tar::Entries<'_, R>,
    ) -> Result<FileTable, TreeError> {
        let mut first =
            entries.next().ok_or(TreeError::TableMissing)?.map_err(
                |error| extraction(&format!("unreadable first entry: {error}")),
            )?;
        let path = first
            .path()
            .map_err(|_| TreeError::TableMissing)?
            .to_string_lossy()
            .into_owned();
        if path != TABLE_NAME {
            return Err(TreeError::TableMissing);
        }
        let mut bytes = Vec::new();
        first.read_to_end(&mut bytes).map_err(|error| {
            extraction(&format!("unreadable table: {error}"))
        })?;
        FileTable::parse(&bytes)
    }

    const fn classify_kind(entry_type: tar::EntryType) -> EntryKind {
        use tar::EntryType;
        match entry_type {
            EntryType::Regular | EntryType::Continuous => EntryKind::File,
            EntryType::Directory => EntryKind::Directory,
            EntryType::Symlink => EntryKind::Symlink,
            EntryType::Link => EntryKind::Hardlink,
            EntryType::Fifo => EntryKind::Fifo,
            EntryType::Char | EntryType::Block => EntryKind::Device,
            EntryType::GNULongName
            | EntryType::GNULongLink
            | EntryType::XHeader
            | EntryType::XGlobalHeader => EntryKind::LongNameOverride,
            _ => EntryKind::Unknown,
        }
    }

    /// `tar` exposes a member's path only as an owned value or a short-lived
    /// Cow, so extraction threads an owned copy the classifier can borrow. A
    /// non-UTF-8 path is preserved as-is so the classifier refuses it under its
    /// own charset policy rather than being lost to a lossy conversion here.
    fn owned_path<R: Read>(entry: &tar::Entry<'_, R>) -> std::ffi::OsString {
        os_str_from_bytes(entry.path_bytes())
    }

    fn os_str_from_bytes(
        bytes: std::borrow::Cow<'_, [u8]>,
    ) -> std::ffi::OsString {
        use std::os::unix::ffi::OsStringExt as _;
        std::ffi::OsString::from_vec(bytes.into_owned())
    }

    /// The admission mask only ever yields 0755/0644, well within a mode, so a
    /// value that would not fit is a defect rather than something to truncate.
    fn mode_bits(mode: u32) -> Mode {
        Mode::from_raw_mode(
            rustix::fs::RawMode::try_from(mode).unwrap_or(0o644),
        )
    }

    fn open_dir(path: &Path) -> std::io::Result<OwnedFd> {
        Ok(rustix::fs::open(
            path,
            OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC,
            Mode::empty(),
        )?)
    }

    /// Walk to the parent of `path` through a fresh fd per component, each
    /// opened `O_NOFOLLOW | O_DIRECTORY`, so a symlink planted at any component
    /// is refused rather than traversed.
    fn open_parent(
        root: &OwnedFd,
        path: &str,
    ) -> Result<(OwnedFd, String), TreeError> {
        let mut components: Vec<&str> =
            path.split('/').filter(|part| !part.is_empty()).collect();
        let leaf = components
            .pop()
            .ok_or_else(|| extraction("an entry has an empty path"))?
            .to_owned();
        let mut current = clone_fd(root)?;
        for component in components {
            current = rustix::fs::openat(
                current.as_fd(),
                component,
                OFlags::RDONLY
                    | OFlags::DIRECTORY
                    | OFlags::NOFOLLOW
                    | OFlags::CLOEXEC,
                Mode::empty(),
            )
            .map_err(|error| {
                // O_NOFOLLOW makes a symlink component fail ELOOP, and
                // O_DIRECTORY makes it ENOTDIR — either way a component that is
                // not a real directory we may descend is a containment breach,
                // not an ordinary I/O error.
                if matches!(
                    error,
                    rustix::io::Errno::LOOP | rustix::io::Errno::NOTDIR
                ) {
                    TreeError::PathEscape {
                        entry: path.to_owned(),
                    }
                } else {
                    extraction(&format!(
                        "cannot descend into '{component}': {error}"
                    ))
                }
            })?;
        }
        Ok((current, leaf))
    }

    fn clone_fd(fd: &OwnedFd) -> Result<OwnedFd, TreeError> {
        fd.try_clone().map_err(|error| {
            extraction(&format!("cannot duplicate a directory fd: {error}"))
        })
    }

    fn make_dir(
        root: &OwnedFd,
        path: &str,
        mode: u32,
    ) -> Result<(), TreeError> {
        let (parent, leaf) = open_parent(root, path)?;
        // A generation is fresh by construction and deterministic assembly
        // lists a directory before its contents, so `EEXIST` here means a
        // duplicate directory entry — a defect, not something to tolerate.
        rustix::fs::mkdirat(parent.as_fd(), leaf.as_str(), mode_bits(mode))
            .map_err(|error| {
                extraction(&format!("cannot create '{path}': {error}"))
            })
    }

    fn write_file<R: Read>(
        root: &OwnedFd,
        path: &str,
        mode: u32,
        entry: &mut tar::Entry<'_, R>,
        row: &TableRow,
    ) -> Result<u64, TreeError> {
        let (parent, leaf) = open_parent(root, path)?;
        let file = rustix::fs::openat(
            parent.as_fd(),
            leaf.as_str(),
            OFlags::WRONLY
                | OFlags::CREATE
                | OFlags::EXCL
                | OFlags::NOFOLLOW
                | OFlags::CLOEXEC,
            mode_bits(mode),
        )
        .map_err(|error| {
            extraction(&format!("cannot create '{path}': {error}"))
        })?;
        let mut file = std::fs::File::from(file);

        let mut hasher = Sha256::new();
        let mut buffer = vec![0_u8; CHUNK];
        let mut written = 0_u64;
        loop {
            let read = entry.read(&mut buffer).map_err(|error| {
                extraction(&format!("cannot read '{path}': {error}"))
            })?;
            if read == 0 {
                break;
            }
            hasher.update(&buffer[..read]);
            std::io::Write::write_all(&mut file, &buffer[..read]).map_err(
                |error| extraction(&format!("cannot write '{path}': {error}")),
            )?;
            written += read as u64;
        }

        let digest = hex(&hasher.finalize());
        if row.sha256.as_deref() != Some(digest.as_str()) {
            return Err(extraction(&format!(
                "'{path}' does not match its file-table digest"
            )));
        }
        if written != row.size {
            return Err(extraction(&format!(
                "'{path}' is {written} bytes, the table records {}",
                row.size
            )));
        }
        Ok(written)
    }

    fn make_symlink(
        root: &OwnedFd,
        path: &str,
        row: &TableRow,
    ) -> Result<(), TreeError> {
        let target = row.link_target.as_deref().ok_or_else(|| {
            extraction(&format!("'{path}' is a symlink with no table target"))
        })?;
        let (parent, leaf) = open_parent(root, path)?;
        rustix::fs::symlinkat(target, parent.as_fd(), leaf.as_str()).map_err(
            |error| {
                extraction(&format!("cannot create symlink '{path}': {error}"))
            },
        )
    }

    fn hex(bytes: &[u8]) -> String {
        use std::fmt::Write as _;
        bytes.iter().fold(String::new(), |mut acc, byte| {
            let _ = write!(acc, "{byte:02x}");
            acc
        })
    }

    fn refusal_to_tree_error(
        refusal: crate::launch::core::tree_entry::EntryRefusal,
    ) -> TreeError {
        use crate::launch::core::tree_entry::EntryRefusal;
        match refusal {
            EntryRefusal::AbsolutePath
            | EntryRefusal::ParentComponent
            | EntryRefusal::EscapingLinkTarget
            | EntryRefusal::DriveOrUncPrefix => TreeError::PathEscape {
                entry: format!("{refusal:?}"),
            },
            other => extraction(&format!("rejected entry: {other:?}")),
        }
    }

    fn extraction(detail: &str) -> TreeError {
        TreeError::Extraction {
            detail: detail.to_owned(),
        }
    }
}

#[cfg(not(unix))]
pub fn extract_archive<R: std::io::Read>(
    _reader: R,
    _dest: &std::path::Path,
    _limits: &crate::launch::core::tree_entry::ExtractionLimits,
) -> Result<Extracted, crate::launch::core::tree::TreeError> {
    unimplemented!("tree extraction is a Unix-only path")
}

#[cfg(all(test, unix))]
#[allow(clippy::expect_used)]
mod tests {
    use std::io::Write as _;
    use std::path::Path;

    use sha2::{Digest as _, Sha256};

    use crate::launch::core::tree::TreeError;
    use crate::launch::core::tree_entry::ExtractionLimits;

    use super::super::table::TABLE_NAME;
    use super::{extract_archive, unix};

    /// A member to place in a synthetic archive.
    enum Member {
        File {
            path: &'static str,
            mode: u32,
            body: Vec<u8>,
        },
        Dir {
            path: &'static str,
        },
        Symlink {
            path: &'static str,
            target: &'static str,
        },
        /// A raw member of an arbitrary type, so a test can plant a hostile
        /// entry the admission rules must refuse.
        Raw {
            path: &'static str,
            entry_type: tar::EntryType,
            link: Option<&'static str>,
        },
    }

    fn hex(bytes: &[u8]) -> String {
        use std::fmt::Write as _;
        bytes.iter().fold(String::new(), |mut acc, byte| {
            let _ = write!(acc, "{byte:02x}");
            acc
        })
    }

    /// Build the `.files` table for the members that carry a row, then the
    /// gzipped tar with the table first. A test can override the table to
    /// simulate a producer/consumer disagreement.
    fn archive(members: &[Member], table_override: Option<&str>) -> Vec<u8> {
        use std::fmt::Write as _;
        let mut table = String::from("version 1\n");
        for member in members {
            match member {
                Member::File { path, mode, body } => {
                    let digest = hex(&Sha256::digest(body));
                    let _ = writeln!(
                        table,
                        "f\t{mode:o}\t{}\t{digest}\t{path}",
                        body.len()
                    );
                }
                Member::Dir { path } => {
                    let _ = writeln!(table, "d\t755\t0\t-\t{path}");
                }
                Member::Symlink { path, target } => {
                    let _ = writeln!(table, "l\t777\t0\t-\t{path}\t{target}");
                }
                Member::Raw { .. } => {}
            }
        }
        let table = table_override.map_or(table, str::to_owned);

        let mut builder = tar::Builder::new(Vec::new());
        append_file(&mut builder, TABLE_NAME, 0o644, table.as_bytes());
        for member in members {
            match member {
                Member::File { path, mode, body } => {
                    append_file(&mut builder, path, *mode, body);
                }
                Member::Dir { path } => {
                    let mut header = tar::Header::new_gnu();
                    header.set_entry_type(tar::EntryType::Directory);
                    header.set_mode(0o755);
                    header.set_size(0);
                    builder
                        .append_data(&mut header, path, std::io::empty())
                        .expect("append dir");
                }
                Member::Symlink { path, target } => {
                    let mut header = tar::Header::new_gnu();
                    header.set_entry_type(tar::EntryType::Symlink);
                    header.set_mode(0o777);
                    header.set_size(0);
                    builder
                        .append_link(&mut header, path, target)
                        .expect("append symlink");
                }
                Member::Raw {
                    path,
                    entry_type,
                    link,
                } => {
                    let mut header = tar::Header::new_gnu();
                    header.set_entry_type(*entry_type);
                    header.set_mode(0o644);
                    header.set_size(0);
                    if let Some(link) = link {
                        builder
                            .append_link(&mut header, path, link)
                            .expect("append raw link");
                    } else {
                        header.set_path(path).expect("set raw path");
                        header.set_cksum();
                        builder
                            .append(&header, std::io::empty())
                            .expect("append raw");
                    }
                }
            }
        }
        let tar = builder.into_inner().expect("finish tar");
        let mut encoder = flate2::write::GzEncoder::new(
            Vec::new(),
            flate2::Compression::fast(),
        );
        encoder.write_all(&tar).expect("gzip");
        encoder.finish().expect("finish gzip")
    }

    fn append_file(
        builder: &mut tar::Builder<Vec<u8>>,
        path: &str,
        mode: u32,
        body: &[u8],
    ) {
        let mut header = tar::Header::new_gnu();
        header.set_entry_type(tar::EntryType::Regular);
        header.set_mode(mode);
        header.set_size(body.len() as u64);
        builder
            .append_data(&mut header, path, body)
            .expect("append file");
    }

    fn gz_reader(bytes: &[u8]) -> flate2::read::GzDecoder<&[u8]> {
        flate2::read::GzDecoder::new(bytes)
    }

    const fn limits() -> ExtractionLimits {
        ExtractionLimits {
            uncompressed_size: 1 << 20,
            entry_count: 100,
        }
    }

    fn extract_into(
        archive_bytes: &[u8],
        dest: &Path,
    ) -> Result<unix::Extracted, TreeError> {
        extract_archive(gz_reader(archive_bytes), dest, &limits())
    }

    #[test]
    fn a_well_formed_archive_lands_every_member() {
        let members = [
            Member::Dir { path: "lib" },
            Member::File {
                path: "lib/data",
                mode: 0o644,
                body: b"payload".to_vec(),
            },
            Member::File {
                path: "shell",
                mode: 0o755,
                body: b"#!/bin/sh\n".to_vec(),
            },
            Member::Symlink {
                path: "lib/current",
                target: "data",
            },
        ];
        let bytes = archive(&members, None);
        let dest = tempfile::tempdir().expect("tempdir");
        let extracted =
            extract_into(&bytes, dest.path()).expect("extraction succeeds");
        assert_eq!(extracted.entry_count, 4);

        assert_eq!(
            std::fs::read(dest.path().join("lib/data")).expect("data"),
            b"payload"
        );
        let link = std::fs::read_link(dest.path().join("lib/current"))
            .expect("symlink");
        assert_eq!(link, Path::new("data"));
    }

    #[test]
    fn an_archive_whose_first_member_is_not_the_table_is_refused() {
        // Hand-roll an archive with a file before the table.
        let mut builder = tar::Builder::new(Vec::new());
        append_file(&mut builder, "lib/data", 0o644, b"x");
        append_file(&mut builder, TABLE_NAME, 0o644, b"version 1\n");
        let tar = builder.into_inner().expect("tar");
        let mut encoder = flate2::write::GzEncoder::new(
            Vec::new(),
            flate2::Compression::fast(),
        );
        encoder.write_all(&tar).expect("gz");
        let bytes = encoder.finish().expect("gz");

        let dest = tempfile::tempdir().expect("tempdir");
        assert!(matches!(
            extract_into(&bytes, dest.path()),
            Err(TreeError::TableMissing)
        ));
    }

    #[test]
    fn a_member_disagreeing_with_the_table_is_refused_during_extraction() {
        let members = [Member::File {
            path: "lib/data",
            mode: 0o644,
            body: b"honest".to_vec(),
        }];
        // Rewrite the table to record a digest for different bytes.
        let forged_digest = hex(&Sha256::digest(b"substituted"));
        let table =
            format!("version 1\nf\t644\t6\t{forged_digest}\tlib/data\n");
        let bytes = archive(&members, Some(&table));
        let dest = tempfile::tempdir().expect("tempdir");
        assert!(matches!(
            extract_into(&bytes, dest.path()),
            Err(TreeError::Extraction { .. })
        ));
    }

    #[test]
    fn each_hostile_entry_shape_is_refused() {
        // A `../` path cannot be planted through `tar::Builder`, which refuses
        // to write one — that case is covered by the pure admission test.
        let cases: Vec<(&str, Member)> = vec![
            (
                "escaping symlink",
                Member::Symlink {
                    path: "here",
                    target: "../../etc/passwd",
                },
            ),
            (
                "hardlink",
                Member::Raw {
                    path: "hard",
                    entry_type: tar::EntryType::Link,
                    link: Some("lib/data"),
                },
            ),
            (
                "fifo",
                Member::Raw {
                    path: "pipe",
                    entry_type: tar::EntryType::Fifo,
                    link: None,
                },
            ),
        ];
        for (label, hostile) in cases {
            let members = [hostile];
            let bytes = archive(&members, None);
            let dest = tempfile::tempdir().expect("tempdir");
            let outcome = extract_into(&bytes, dest.path());
            assert!(
                outcome.is_err(),
                "{label} was extracted rather than refused"
            );
        }
    }

    #[test]
    fn a_symlink_then_traverse_chain_is_refused_at_the_open() {
        // The admission rules pass both members in isolation: `lib -> sibling`
        // stays inside the tree, and `lib/escape` is a plain relative path.
        // Only the openat chain, refusing to descend through the symlink, stops
        // the file being written outside the tree. This is the case the whole
        // O_NOFOLLOW mechanism exists for.
        let members = [
            Member::Symlink {
                path: "lib",
                target: "sibling",
            },
            Member::File {
                path: "lib/escape",
                mode: 0o644,
                body: b"escaped".to_vec(),
            },
        ];
        let bytes = archive(&members, None);
        let dest = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir(dest.path().join("sibling")).expect("sibling");
        let outcome = extract_into(&bytes, dest.path());
        assert!(
            matches!(outcome, Err(TreeError::PathEscape { .. })),
            "a symlink-then-traverse chain was not refused: {outcome:?}"
        );
        assert!(
            !dest.path().join("sibling/escape").exists(),
            "the file escaped through the symlink"
        );
    }

    #[test]
    fn an_over_size_tree_aborts_before_filling_the_disk() {
        let members = [Member::File {
            path: "big",
            mode: 0o644,
            body: vec![0_u8; 4096],
        }];
        let bytes = archive(&members, None);
        let dest = tempfile::tempdir().expect("tempdir");
        let tight = ExtractionLimits {
            uncompressed_size: 100,
            entry_count: 100,
        };
        assert!(matches!(
            extract_archive(gz_reader(&bytes), dest.path(), &tight),
            Err(TreeError::Extraction { .. })
        ));
    }
}
