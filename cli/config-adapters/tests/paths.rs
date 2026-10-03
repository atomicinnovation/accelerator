//! The path checks' filesystem port over the real filesystem.
#![cfg(unix)]

use std::os::unix::fs::symlink;
use std::path::Path;
use std::path::PathBuf;

use config::consent::ExecutablePaths as _;
use config_adapters::paths::SystemExecutablePaths;

type TestError = Box<dyn std::error::Error>;

#[test]
fn a_dangling_symlinks_target_is_reported_exactly_as_stored(
) -> Result<(), TestError> {
    let work = tempfile::tempdir()?;
    let link = work.path().join("chromium");
    symlink("../gone/chromium", &link)?;

    assert!(SystemExecutablePaths.exists(&link));
    assert_eq!(
        SystemExecutablePaths.link_target(&link),
        Some(PathBuf::from("../gone/chromium"))
    );
    assert_eq!(SystemExecutablePaths.canonicalise(&link), None);
    Ok(())
}

#[test]
fn a_file_under_a_symlinked_parent_canonicalises_to_its_real_location(
) -> Result<(), TestError> {
    let work = tempfile::tempdir()?;
    let real = work.path().join("real");
    std::fs::create_dir(&real)?;
    std::fs::write(real.join("chromium"), "")?;
    symlink(&real, work.path().join("alias"))?;
    let aliased = work.path().join("alias/chromium");

    assert_eq!(SystemExecutablePaths.link_target(&aliased), None);
    assert_eq!(
        SystemExecutablePaths.canonicalise(&aliased),
        Some(real.join("chromium").canonicalize()?)
    );
    Ok(())
}

#[test]
fn a_missing_entry_does_not_exist() {
    assert!(!SystemExecutablePaths.exists(Path::new("/no/such/chromium")));
    assert_eq!(
        SystemExecutablePaths.link_target(Path::new("/no/such/chromium")),
        None
    );
}
