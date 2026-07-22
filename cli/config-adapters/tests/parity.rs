//! Direct declared-value assertions for depth >=3, for inline and block arrays
//! and typed sequences, for the value-encoding divergences, and for the
//! fail-loud malformed and characterised adversarial cases. The differential
//! shell-out oracle against the bash reader retired with the bash reader
//! itself (0167 Phase 7); these are the declared-value tests the divergence
//! records (10-12) name.

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use config::{ConfigAccess, ConfigError, ConfigService, Key, Resolved, Value};
use config_adapters::{render_resolved, FileConfigStore};

static COUNTER: AtomicU64 = AtomicU64::new(0);

type Store = ConfigService<FileConfigStore, FileConfigStore>;

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/configs")
}

fn materialise(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!(
        "parity-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(dir.join(".git")).unwrap();
    copy_dir(
        &fixtures().join(name).join(".accelerator"),
        &dir.join(".accelerator"),
    );
    dir
}

fn copy_dir(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let dest = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &dest);
        } else {
            std::fs::copy(entry.path(), dest).unwrap();
        }
    }
}

fn service(dir: &Path) -> Store {
    let root = FileConfigStore::discover_root(dir);
    let store = FileConfigStore::at(root);
    ConfigService::new(store.clone(), store)
}

fn resolve(dir: &Path, key: &str) -> Result<Resolved, ConfigError> {
    service(dir).get(&Key::parse(key).unwrap(), None)
}

fn rendered(dir: &Path, key: &str) -> String {
    render_resolved(&resolve(dir, key).unwrap())
}

#[test]
fn depth_beyond_two_resolves_declared_scalar_values() {
    let dir = materialise("deep");
    assert_eq!(rendered(&dir, "a.b.c"), "three");
    assert_eq!(rendered(&dir, "a.b.d.e"), "four");
}

#[test]
fn inline_and_nested_arrays_resolve_to_typed_sequences() {
    let deep = materialise("deep");
    assert_eq!(
        resolve(&deep, "a.b.items").unwrap(),
        Resolved::Found(Value::Sequence(vec![
            config::Scalar::String("foo".to_owned()),
            config::Scalar::String("bar".to_owned()),
        ]))
    );

    let arrays = materialise("arrays");
    assert_eq!(
        resolve(&arrays, "review.core_lenses").unwrap(),
        Resolved::Found(Value::Sequence(vec![
            config::Scalar::String("architecture".to_owned()),
            config::Scalar::String("code-quality".to_owned()),
        ]))
    );
    assert_eq!(
        resolve(&arrays, "review.disabled_lenses").unwrap(),
        Resolved::Found(Value::Sequence(Vec::new()))
    );
    assert_eq!(rendered(&arrays, "review.disabled_lenses"), "[]");
}

#[test]
fn an_absent_array_key_defaults_to_the_same_sequence_shape() {
    assert!(matches!(
        config::catalogue::default_for("review.core_lenses"),
        Some(Value::Sequence(_))
    ));
}

#[test]
fn value_encodings_resolve_to_their_declared_divergent_forms() {
    let dir = materialise("encodings");
    let cases = [
        ("enc.id_pattern", ""),
        ("enc.leading_bracket", "[a, b]"),
        ("enc.double_quoted", "value"),
        ("enc.quoted_element", "[a, b]"),
        ("enc.null_token", ""),
        ("enc.tilde_null", ""),
        ("enc.bool_case", "true"),
        ("enc.zero_padded", "7"),
        ("enc.float_int", "1"),
        ("enc.exp", "1000"),
        ("enc.plus", "5"),
        ("enc.trailing_comment", "meta/work"),
    ];
    for (key, expected) in cases {
        assert_eq!(rendered(&dir, key), expected, "encoding {key}");
    }
}

#[test]
fn a_block_authored_array_diverges_from_the_bash_found_empty() {
    let dir = materialise("block-array");
    assert_eq!(
        resolve(&dir, "review.core_lenses").unwrap(),
        Resolved::Found(Value::Sequence(vec![
            config::Scalar::String("architecture".to_owned()),
            config::Scalar::String("code-quality".to_owned()),
        ]))
    );
}

#[test]
fn malformed_frontmatter_is_fail_loud_where_bash_degrades() {
    let team = materialise("malformed-team");
    assert!(matches!(
        resolve(&team, "paths.work"),
        Err(ConfigError::MalformedFrontmatter { .. })
    ));

    let local = materialise("malformed-local");
    assert!(matches!(
        resolve(&local, "paths.work"),
        Err(ConfigError::MalformedFrontmatter { .. })
    ));
}

#[test]
fn adversarial_input_terminates_with_an_error_not_a_hang() {
    for name in ["adversarial-deep", "adversarial-aliases"] {
        let dir = materialise(name);
        let (tx, rx) = mpsc::channel();
        thread::spawn(move || {
            let outcome = resolve(&dir, "leaf").is_err();
            let _ = tx.send(outcome);
        });
        let is_err =
            rx.recv_timeout(Duration::from_secs(30))
                .unwrap_or_else(|_| {
                    panic!(
                        "{name} parse did not terminate within the time bound"
                    )
                });
        assert!(
            is_err,
            "{name} was expected to characterise as a ConfigError"
        );
    }
}
