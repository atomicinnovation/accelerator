//! A permanent regression guard: the migrate/migrate-adapters/migrate-cli
//! crates never shell out to `awk`, and no `.awk` file exists under them —
//! trivially true today (there is no awk in Rust), pinned so it stays true.

use std::path::Path;

type TestError = Box<dyn std::error::Error>;

fn crate_source_files(
    crate_dir: &str,
) -> Result<Vec<std::path::PathBuf>, TestError> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join(crate_dir)
        .join("src");
    let mut files = Vec::new();
    let mut stack = vec![root];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else {
                files.push(path);
            }
        }
    }
    Ok(files)
}

#[test]
fn no_migrate_crate_shells_out_to_awk_or_carries_an_awk_file(
) -> Result<(), TestError> {
    for crate_dir in ["migrate", "migrate-adapters", "migrate-cli"] {
        for path in crate_source_files(crate_dir)? {
            assert_ne!(
                path.extension().and_then(std::ffi::OsStr::to_str),
                Some("awk"),
                "{} carries an .awk file",
                path.display()
            );
            let content = std::fs::read_to_string(&path)?;
            assert!(
                !content.to_lowercase().contains("awk"),
                "{} mentions awk",
                path.display()
            );
        }
    }
    Ok(())
}
