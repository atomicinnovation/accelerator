use std::collections::BTreeMap;
use std::path::Path;

use vcs_test_support::colocation::JjColocation;
use vcs_test_support::family::CopyableFamily;
use vcs_test_support::family::SharedFamily;
use vcs_test_support::stalled_migration::manifest::StalledMigrationManifest;
use vcs_test_support::stalled_migration::ParentShape;
use vcs_test_support::stalled_migration::Stall;
use vcs_test_support::stalled_migration::StallKey;
use vcs_test_support::stalled_migration::StalledMigrations;

type TestError = Box<dyn std::error::Error>;

const PURE_MERGE: StallKey = StallKey {
    colocation: JjColocation::Pure,
    shape: ParentShape::MergeWithSibling,
};

const COLOCATED_SINGLE: StallKey = StallKey {
    colocation: JjColocation::Colocated,
    shape: ParentShape::SingleParent,
};

fn stall(base: &Path, key: StallKey, change: &str) -> Stall {
    Stall {
        root: base.join(key.to_string()),
        base: format!("{change}base"),
        sibling: format!("{change}sibling"),
    }
}

fn two_stalls(base: &Path) -> BTreeMap<StallKey, Stall> {
    BTreeMap::from([
        (PURE_MERGE, stall(base, PURE_MERGE, "k")),
        (COLOCATED_SINGLE, stall(base, COLOCATED_SINGLE, "z")),
    ])
}

#[test]
fn a_written_manifest_reads_back_unchanged() -> Result<(), TestError> {
    let directory = tempfile::tempdir()?;
    let written = StalledMigrationManifest {
        stalls: two_stalls(directory.path()),
    };
    written.write(directory.path())?;

    let read = StalledMigrationManifest::read(directory.path())?;

    assert_eq!(read.stalls, written.stalls);
    Ok(())
}

#[test]
fn a_moved_build_s_manifest_names_its_stalls_where_they_now_are(
) -> Result<(), TestError> {
    let directory = tempfile::tempdir()?;
    let built = directory.path().join("built");
    let moved = directory.path().join("moved");
    std::fs::create_dir(&built)?;
    StalledMigrationManifest {
        stalls: two_stalls(&built),
    }
    .write(&built)?;
    std::fs::rename(&built, &moved)?;

    let read = StalledMigrationManifest::read(&moved)?;

    assert_eq!(read.stalls, two_stalls(&moved));
    Ok(())
}

#[test]
fn reading_a_directory_without_a_manifest_fails_naming_it(
) -> Result<(), TestError> {
    let directory = tempfile::tempdir()?;

    let error = StalledMigrationManifest::read(directory.path())
        .err()
        .ok_or("it should fail")?;

    assert!(
        error
            .to_string()
            .contains(&directory.path().display().to_string()),
        "{error}"
    );
    Ok(())
}

#[test]
fn a_manifest_line_naming_no_stall_key_is_refused() -> Result<(), TestError> {
    let directory = tempfile::tempdir()?;
    std::fs::write(
        directory.path().join("stalled-migrations.tsv"),
        "pure-octopus\tb\ts\n",
    )?;

    let error = StalledMigrationManifest::read(directory.path())
        .err()
        .ok_or("it should fail")?;

    assert!(error.to_string().contains("pure-octopus"), "{error}");
    Ok(())
}

#[test]
fn a_shared_build_is_adopted_from_its_manifest() -> Result<(), TestError> {
    let directory = tempfile::tempdir()?;
    let base = directory.path().canonicalize()?;
    StalledMigrationManifest {
        stalls: two_stalls(&base),
    }
    .write(&base)?;

    let adopted = StalledMigrations::adopt(&base)?;

    assert_eq!(adopted.stalls, two_stalls(&base));
    Ok(())
}

#[test]
fn a_part_is_its_stall_s_directory_alone() -> Result<(), TestError> {
    let directory = tempfile::tempdir()?;

    let entries = StalledMigrations::entries(&PURE_MERGE, directory.path())?;

    assert_eq!(entries, ["pure-merge-with-sibling"]);
    Ok(())
}

#[test]
fn a_rebased_set_keeps_only_the_copied_stall_and_its_change_ids() {
    let from = Path::new("/shared");
    let onto = Path::new("/workbench");
    let shared = StalledMigrations {
        base: from.to_path_buf(),
        stalls: two_stalls(from),
    };

    let rebased =
        shared.rebased(from, onto, &["pure-merge-with-sibling".to_owned()]);

    assert_eq!(rebased.base, onto);
    assert_eq!(
        rebased.stalls,
        BTreeMap::from([(PURE_MERGE, stall(onto, PURE_MERGE, "k"))])
    );
}

#[test]
fn looking_up_a_stall_the_set_lacks_names_its_key() -> Result<(), TestError> {
    let shared = StalledMigrations {
        base: Path::new("/shared").to_path_buf(),
        stalls: BTreeMap::new(),
    };

    let error = shared.stall(PURE_MERGE).err().ok_or("it should fail")?;

    assert!(
        error.to_string().contains("pure-merge-with-sibling"),
        "{error}"
    );
    Ok(())
}
