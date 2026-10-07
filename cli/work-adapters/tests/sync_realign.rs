//! Realigning sync baselines after a whole-corpus re-render: a `Synced`
//! entry advances to the re-rendered digest, a diverged one keeps its
//! pending push.

use std::cell::RefCell;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::rc::Rc;

use corpus::scan::FileReader;
use corpus::store::AtomicWrite;
use corpus::store::StoreError;
use tempfile::TempDir;
use work_adapters::sync::baseline::Baseline;
use work_adapters::sync::digest;
use work_adapters::sync::realign::realign_baselines;
use work_adapters::sync::realign::RealignError;

type TestError = Box<dyn std::error::Error>;

struct DiskReader;

impl FileReader for DiskReader {
    fn read(&self, path: &Path) -> Result<Option<String>, kernel::Error> {
        match fs::read_to_string(path) {
            Ok(content) => Ok(Some(content)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                Ok(None)
            }
            Err(error) => Err(kernel::Error::Failed(error.to_string())),
        }
    }
}

struct RefusingReader;

impl FileReader for RefusingReader {
    fn read(&self, _path: &Path) -> Result<Option<String>, kernel::Error> {
        Err(kernel::Error::Failed("permission denied".to_owned()))
    }
}

struct DiskWriter;

impl AtomicWrite for DiskWriter {
    fn write(&self, path: &Path, bytes: &[u8]) -> Result<(), StoreError> {
        fs::write(path, bytes).map_err(|error| StoreError::Io {
            path: path.display().to_string(),
            detail: error.to_string(),
        })
    }
}

fn work_item(id: &str, title: &str) -> String {
    format!("---\nid: {id}\ntitle: {title}\n---\nBody\n")
}

fn quoted_work_item(id: &str, title: &str) -> String {
    format!("---\nid: \"{id}\"\ntitle: \"{title}\"\n---\nBody\n")
}

fn seed_baseline(
    integrations: &Path,
    integration: &str,
    id: &str,
    local_hash: &str,
) -> Result<PathBuf, TestError> {
    let dir = integrations.join(integration);
    fs::create_dir_all(&dir)?;
    let path = dir.join("last-sync.json");
    fs::write(
        &path,
        format!(
            "{{\"timestamp\":1,\"items\":{{\"{id}\":{{\
             \"remote_updated_at\":\"2026-06-01T00:00:00Z\",\
             \"remote_hash\":\"rh\",\"local_hash\":\"{local_hash}\"}}}}}}\n"
        ),
    )?;
    Ok(path)
}

fn local_hash(baseline: &Path, id: &str) -> Result<Option<String>, TestError> {
    let content = fs::read_to_string(baseline)?;
    let (baseline, _) = Baseline::read(Some(&content));
    Ok(baseline.get(id).map(|entry| entry.local_hash.clone()))
}

fn disk_writer_at(_dir: &Path) -> Box<dyn AtomicWrite> {
    Box::new(DiskWriter)
}

struct Corpus {
    _dir: TempDir,
    integrations: PathBuf,
    item: PathBuf,
}

fn re_rendered_corpus(original: &str) -> Result<Corpus, TestError> {
    let dir = TempDir::new()?;
    let integrations = dir.path().join("integrations");
    fs::create_dir_all(&integrations)?;
    let item = dir.path().join("meta/work/0001-x.md");
    fs::create_dir_all(item.parent().ok_or("no parent")?)?;
    fs::write(&item, original)?;
    Ok(Corpus {
        _dir: dir,
        integrations,
        item,
    })
}

#[test]
fn a_synced_entry_advances_to_the_re_rendered_digest() -> Result<(), TestError>
{
    let original = work_item("0001", "Synced");
    let corpus = re_rendered_corpus(&original)?;
    let baseline = seed_baseline(
        &corpus.integrations,
        "linear",
        "0001",
        &digest::local(&original)?,
    )?;
    let rendered = quoted_work_item("0001", "Synced");
    fs::write(&corpus.item, &rendered)?;

    let realigned = realign_baselines(
        &corpus.integrations,
        &[(corpus.item.clone(), original)],
        &DiskReader,
        &disk_writer_at,
    )?;

    assert_eq!(realigned, 1);
    assert_eq!(
        local_hash(&baseline, "0001")?,
        Some(digest::local(&rendered)?)
    );
    Ok(())
}

#[test]
fn a_diverged_entry_keeps_its_baseline() -> Result<(), TestError> {
    let original = work_item("0001", "Modified");
    let corpus = re_rendered_corpus(&original)?;
    let stale = "0".repeat(64);
    let baseline =
        seed_baseline(&corpus.integrations, "linear", "0001", &stale)?;
    fs::write(&corpus.item, quoted_work_item("0001", "Modified"))?;

    let realigned = realign_baselines(
        &corpus.integrations,
        &[(corpus.item.clone(), original)],
        &DiskReader,
        &disk_writer_at,
    )?;

    assert_eq!(realigned, 0);
    assert_eq!(local_hash(&baseline, "0001")?, Some(stale));
    Ok(())
}

#[test]
fn an_unchanged_digest_is_not_counted() -> Result<(), TestError> {
    let original = quoted_work_item("0001", "Canonical");
    let corpus = re_rendered_corpus(&original)?;
    seed_baseline(
        &corpus.integrations,
        "linear",
        "0001",
        &digest::local(&original)?,
    )?;

    let realigned = realign_baselines(
        &corpus.integrations,
        &[(corpus.item.clone(), original)],
        &DiskReader,
        &disk_writer_at,
    )?;

    assert_eq!(realigned, 0);
    Ok(())
}

#[test]
fn an_integration_without_a_baseline_realigns_nothing() -> Result<(), TestError>
{
    let original = work_item("0001", "Untracked");
    let corpus = re_rendered_corpus(&original)?;
    fs::create_dir_all(corpus.integrations.join("jira"))?;
    fs::write(&corpus.item, quoted_work_item("0001", "Untracked"))?;

    let realigned = realign_baselines(
        &corpus.integrations,
        &[(corpus.item.clone(), original)],
        &DiskReader,
        &disk_writer_at,
    )?;

    assert_eq!(realigned, 0);
    assert!(!corpus.integrations.join("jira/last-sync.json").exists());
    Ok(())
}

#[test]
fn each_baseline_is_written_from_its_own_directory() -> Result<(), TestError> {
    let original = work_item("0001", "Twice");
    let corpus = re_rendered_corpus(&original)?;
    let hash = digest::local(&original)?;
    seed_baseline(&corpus.integrations, "jira", "0001", &hash)?;
    seed_baseline(&corpus.integrations, "linear", "0001", &hash)?;
    fs::write(&corpus.item, quoted_work_item("0001", "Twice"))?;
    let roots = Rc::new(RefCell::new(Vec::new()));
    let recorded = Rc::clone(&roots);
    let writer_at = move |dir: &Path| -> Box<dyn AtomicWrite> {
        recorded.borrow_mut().push(dir.to_path_buf());
        Box::new(DiskWriter)
    };

    let realigned = realign_baselines(
        &corpus.integrations,
        &[(corpus.item.clone(), original)],
        &DiskReader,
        &writer_at,
    )?;

    assert_eq!(realigned, 2);
    let mut roots = roots.borrow().clone();
    roots.sort();
    assert_eq!(
        roots,
        vec![
            corpus.integrations.join("jira"),
            corpus.integrations.join("linear"),
        ]
    );
    Ok(())
}

#[test]
fn an_unlistable_integrations_root_is_a_listing_error() -> Result<(), TestError>
{
    let dir = TempDir::new()?;
    let not_a_directory = dir.path().join("integrations");
    fs::write(&not_a_directory, "")?;
    let expected = fs::read_dir(&not_a_directory)
        .err()
        .ok_or("listing a file must fail")?
        .to_string();

    let result =
        realign_baselines(&not_a_directory, &[], &DiskReader, &disk_writer_at);

    let Err(error @ RealignError::Listing(_)) = result else {
        return Err(format!("expected a listing error, got {result:?}").into());
    };
    assert_eq!(error.to_string(), expected);
    Ok(())
}

#[test]
fn an_unreadable_baseline_is_a_baseline_error() -> Result<(), TestError> {
    let original = work_item("0001", "Unreadable");
    let corpus = re_rendered_corpus(&original)?;
    let baseline =
        seed_baseline(&corpus.integrations, "linear", "0001", "hash")?;

    let result = realign_baselines(
        &corpus.integrations,
        &[(corpus.item.clone(), original)],
        &RefusingReader,
        &disk_writer_at,
    );

    let Err(error @ RealignError::Baseline(_)) = result else {
        return Err(format!("expected a baseline error, got {result:?}").into());
    };
    assert_eq!(
        error.to_string(),
        format!("I/O error on '{}': permission denied", baseline.display())
    );
    Ok(())
}
