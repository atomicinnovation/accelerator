//! Differential parity against the bash reader (depth <=2, scalar keys) plus
//! direct declared-value assertions at depth >=3, for inline arrays and typed
//! sequences, for the value-encoding divergences, and for the fail-loud
//! malformed and characterised adversarial cases.

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use config::catalogue::{
    Default, AGENT_KEYS, PATH_KEYS, REVIEW_KEYS, TEMPLATE_KEYS, WORK_KEYS,
};
use config::{ConfigAccess, ConfigError, ConfigService, Key, Resolved, Value};
use config_adapters::{render_resolved, FileConfigStore, ABSENT_SENTINEL};

static COUNTER: AtomicU64 = AtomicU64::new(0);

type Store = ConfigService<FileConfigStore, FileConfigStore>;

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/configs")
}

fn scripts_dir() -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../scripts")
        .canonicalize()
        .unwrap_or_else(|_| {
            panic!(
                "scripts/ not found relative to {} — expected the repo \
                 scripts/ directory two levels up",
                env!("CARGO_MANIFEST_DIR")
            )
        });
    assert!(
        dir.join("config-read-value.sh").is_file(),
        "config-read-value.sh missing under {}",
        dir.display()
    );
    dir
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

fn bash_available() -> bool {
    Command::new("bash")
        .arg("-c")
        .arg("exit 0")
        .status()
        .is_ok_and(|status| status.success())
}

fn require_bash() -> bool {
    if bash_available() {
        return true;
    }
    assert!(
        std::env::var_os("CI").is_none()
            && std::env::var_os("GITHUB_ACTIONS").is_none(),
        "bash is required for the parity test under CI"
    );
    eprintln!("skipping bash parity (silent pass): bash unavailable");
    false
}

fn oracle(dir: &Path, scripts: &Path, key: &str, default: &str) -> String {
    let output = Command::new("bash")
        .arg(scripts.join("config-read-value.sh"))
        .arg(key)
        .arg(default)
        .current_dir(dir)
        .env("LANG", "C")
        .env("LC_ALL", "C")
        .env("PWD", dir)
        .output()
        .expect("run config-read-value.sh");
    assert!(
        output.status.success(),
        "bash oracle failed for {key}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout)
        .unwrap()
        .strip_suffix('\n')
        .unwrap_or_default()
        .to_owned()
}

fn scalar_keys() -> Vec<String> {
    let mut keys = Vec::new();
    for (key, _) in PATH_KEYS {
        keys.push((*key).to_owned());
    }
    for key in TEMPLATE_KEYS {
        keys.push((*key).to_owned());
    }
    for (key, _) in WORK_KEYS {
        keys.push((*key).to_owned());
    }
    for (key, default) in REVIEW_KEYS {
        if matches!(default, Default::Scalar(_)) {
            keys.push((*key).to_owned());
        }
    }
    for name in AGENT_KEYS {
        keys.push(format!("agents.{name}"));
    }
    keys
}

#[test]
fn resolution_parity_over_every_scalar_key() {
    if !require_bash() {
        return;
    }
    let scripts = scripts_dir();
    let dir = materialise("all-scalars");
    for key in scalar_keys() {
        let bash = oracle(&dir, &scripts, &key, ABSENT_SENTINEL);
        let rust = rendered(&dir, &key);
        assert_eq!(rust, bash, "resolution drift for {key}");
        assert_ne!(
            rust, ABSENT_SENTINEL,
            "{key} was never genuinely set — parity degraded to a miss"
        );
    }
}

#[test]
fn precedence_and_presence_parity() {
    if !require_bash() {
        return;
    }
    let scripts = scripts_dir();
    let cases = [
        ("precedence", "paths.work", "local-work"),
        ("present-empty-local", "paths.work", ""),
        ("present-null-local", "paths.work", ""),
        ("present-empty-team", "paths.work", ""),
        ("non-scalar-node", "paths.work", ""),
    ];
    for (fixture, key, expected) in cases {
        let dir = materialise(fixture);
        let bash = oracle(&dir, &scripts, key, ABSENT_SENTINEL);
        let rust = rendered(&dir, key);
        assert_eq!(rust, expected, "rust {fixture}/{key}");
        assert_eq!(bash, expected, "bash {fixture}/{key}");
    }
}

#[test]
fn a_genuine_miss_echoes_the_sentinel_on_both_sides() {
    if !require_bash() {
        return;
    }
    let scripts = scripts_dir();
    let dir = materialise("present-empty-team");
    let bash = oracle(&dir, &scripts, "paths.nonexistent", ABSENT_SENTINEL);
    let rust = rendered(&dir, "paths.nonexistent");
    assert_eq!(bash, ABSENT_SENTINEL);
    assert_eq!(rust, ABSENT_SENTINEL);
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
    if !require_bash() {
        return;
    }
    let scripts = scripts_dir();
    let bash = oracle(&dir, &scripts, "review.core_lenses", ABSENT_SENTINEL);
    assert_eq!(bash, "", "bash reads a block array found-empty");
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

    if !require_bash() {
        return;
    }
    let scripts = scripts_dir();
    assert_eq!(
        oracle(&team, &scripts, "paths.work", ABSENT_SENTINEL),
        "local-work",
        "bash skips the malformed team file and reads local"
    );
    assert_eq!(
        oracle(&local, &scripts, "paths.work", ABSENT_SENTINEL),
        "team-work",
        "bash skips the malformed local file and reads team"
    );
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
