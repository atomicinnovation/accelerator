//! CLI-boundary tests for `work sync`.
//!
//! `accelerator-work` is bin-only, so a subprocess cannot inject a fake
//! `TrackerRegistry` and this suite covers only what is reachable from
//! outside the process: provider selection, usage errors, and
//! non-interactivity. The fake-tracker scenarios — the conflict loop,
//! classification stability, the write-bounds boundaries — need a `[lib]`
//! target `work_adapters::sync::run` can be driven through directly.
use std::fs;
use std::net::TcpListener;
use std::path::Path;
use std::process::Command;
use std::process::Stdio;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::sync::Arc;

mod common;

type TestError = Box<dyn std::error::Error>;

fn scratch_repo(
    integration: Option<&str>,
) -> Result<tempfile::TempDir, TestError> {
    let dir = tempfile::Builder::new()
        .prefix("work-cli-sync-")
        .tempdir()?;
    let status = Command::new("git")
        .args(["init", "-q"])
        .current_dir(dir.path())
        .status()?;
    assert!(status.success(), "git init failed");
    fs::create_dir_all(dir.path().join("meta/work"))?;
    fs::create_dir_all(dir.path().join(".accelerator"))?;
    if let Some(integration) = integration {
        fs::write(
            dir.path().join(".accelerator/config.md"),
            format!("---\nwork:\n  integration: {integration}\n---\n"),
        )?;
    }
    Ok(dir)
}

fn configured_repo(config: &str) -> Result<tempfile::TempDir, TestError> {
    let repo = scratch_repo(None)?;
    fs::write(repo.path().join(".accelerator/config.md"), config)?;
    Ok(repo)
}

fn run(dir: &Path, args: &[&str]) -> Result<std::process::Output, TestError> {
    run_with(dir, args, &[])
}

fn run_with(
    dir: &Path,
    args: &[&str],
    environment: &[(&str, &str)],
) -> Result<std::process::Output, TestError> {
    let mut command = Command::new(env!("CARGO_BIN_EXE_accelerator-work"));
    command
        .arg("sync")
        .args(args)
        .current_dir(dir)
        .env("ACCELERATOR_PLUGIN_ROOT", dir)
        .stdin(Stdio::null());
    // A credentialed machine must not resolve a real client and reach the
    // network from the default suite.
    common::scrub_provider_env(&mut command);
    command.envs(environment.iter().copied());
    Ok(command.output()?)
}

#[test]
fn unset_work_integration_exits_73_naming_the_key_and_configure(
) -> Result<(), TestError> {
    let repo = scratch_repo(None)?;
    let output = run(repo.path(), &[])?;
    assert_eq!(output.status.code(), Some(73));
    let stderr = String::from_utf8(output.stderr)?;
    assert!(stderr.contains("work.integration"), "{stderr}");
    assert!(stderr.contains("/accelerator:configure"), "{stderr}");
    Ok(())
}

#[test]
fn an_unrecognised_tracker_exits_73_and_echoes_the_value(
) -> Result<(), TestError> {
    let repo = scratch_repo(Some("bogus-tracker"))?;
    let output = run(repo.path(), &[])?;
    assert_eq!(output.status.code(), Some(73));
    let stderr = String::from_utf8(output.stderr)?;
    assert!(stderr.contains("bogus-tracker"), "{stderr}");
    Ok(())
}

#[test]
fn a_wired_tracker_without_credentials_exits_74() -> Result<(), TestError> {
    let repo = scratch_repo(Some("jira"))?;
    let output = run(repo.path(), &[])?;
    assert_eq!(output.status.code(), Some(74));
    let stderr = String::from_utf8(output.stderr)?;
    assert!(stderr.contains("not usable"), "{stderr}");
    assert!(stderr.contains("jira.site"), "{stderr}");
    Ok(())
}

#[test]
fn trello_also_exits_72_not_73() -> Result<(), TestError> {
    let repo = scratch_repo(Some("trello"))?;
    let output = run(repo.path(), &[])?;
    assert_eq!(output.status.code(), Some(72));
    Ok(())
}

#[test]
fn preview_does_not_bypass_provider_selection() -> Result<(), TestError> {
    let repo = scratch_repo(Some("jira"))?;
    let output = run(repo.path(), &["--preview"])?;
    assert_eq!(output.status.code(), Some(74));
    Ok(())
}

#[test]
fn an_invalid_pull_block_fails_loud_before_discovery() -> Result<(), TestError>
{
    let repo = scratch_repo(Some("linear"))?;
    fs::write(
        repo.path().join(".accelerator/config.md"),
        "---\nwork:\n  integration: linear\nlinear:\n  pull:\n    \
         filters:\n      colour: [red]\n---\n",
    )?;
    let output = run(repo.path(), &["--preview"])?;
    // Exit 1 (a config refusal), not 74 (the credential check that runs
    // later) — proving the block is rejected before discovery.
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8(output.stderr)?;
    assert!(stderr.contains("colour"), "{stderr}");
    assert!(stderr.contains("not supported"), "{stderr}");
    Ok(())
}

#[test]
fn a_jira_project_filter_fails_loud_before_discovery() -> Result<(), TestError>
{
    let repo = scratch_repo(Some("jira"))?;
    fs::write(
        repo.path().join(".accelerator/config.md"),
        "---\nwork:\n  integration: jira\njira:\n  pull:\n    \
         filters:\n      project: [Alpha]\n---\n",
    )?;
    let output = run(repo.path(), &["--preview"])?;
    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8(output.stderr)?;
    assert!(stderr.contains("`project`"), "{stderr}");
    assert!(stderr.contains("label, state, assignee"), "{stderr}");
    Ok(())
}

const JIRA_TOKEN: (&str, &str) = ("ACCELERATOR_JIRA_TOKEN", "fixture-token");

const LINEAR_TOKEN: (&str, &str) =
    ("ACCELERATOR_LINEAR_TOKEN", "fixture-token");

fn jira_config_with(blocks: &str) -> String {
    format!(
        "---\nwork:\n  integration: jira\njira:\n  site: acme\n  \
         email: fixture@example.com\n{blocks}---\n"
    )
}

fn push_only_preview_stderr(
    config: &str,
    token: (&str, &str),
) -> Result<(Option<i32>, String), TestError> {
    let repo = configured_repo(config)?;
    let output =
        run_with(repo.path(), &["--preview", "--push-only"], &[token])?;
    Ok((output.status.code(), String::from_utf8(output.stderr)?))
}

#[test]
fn a_push_ceiling_that_is_not_a_count_is_refused() -> Result<(), TestError> {
    let (code, stderr) = push_only_preview_stderr(
        &jira_config_with("  push:\n    max_items: lots\n"),
        JIRA_TOKEN,
    )?;

    assert_eq!(code, Some(1), "{stderr}");
    assert_eq!(
        stderr,
        "push `max_items` must be a non-negative integer (0 refuses all) \
         or `unlimited` (got `lots`). Fix the `push` block in \
         .accelerator/config.md.\n"
    );
    Ok(())
}

#[test]
fn an_unknown_push_key_is_refused_naming_the_accepted_keys(
) -> Result<(), TestError> {
    let (code, stderr) = push_only_preview_stderr(
        &jira_config_with("  push:\n    bogus: 1\n"),
        JIRA_TOKEN,
    )?;

    assert_eq!(code, Some(1), "{stderr}");
    assert_eq!(
        stderr,
        "the push block key `bogus` is not recognised (accepted: max_items). \
         Fix the `push` block in .accelerator/config.md.\n"
    );
    Ok(())
}

#[test]
fn a_push_only_sync_still_refuses_an_unknown_pull_key() -> Result<(), TestError>
{
    let (code, stderr) = push_only_preview_stderr(
        &jira_config_with("  pull:\n    bogus: 1\n"),
        JIRA_TOKEN,
    )?;

    assert_eq!(code, Some(1), "{stderr}");
    assert_eq!(
        stderr,
        "the pull block key `bogus` is not recognised (accepted: \
         additional_projects, all_projects, filters, max_items, max_pages). \
         Fix the `pull` block in .accelerator/config.md.\n"
    );
    Ok(())
}

#[test]
fn an_empty_pull_block_reads_as_unset_and_reaches_readiness(
) -> Result<(), TestError> {
    let (code, stderr) = push_only_preview_stderr(
        &jira_config_with("  pull: {}\n"),
        JIRA_TOKEN,
    )?;

    assert_eq!(code, Some(74), "{stderr}");
    assert_eq!(
        stderr,
        "work.integration names 'jira', which is wired but not usable — its \
         configuration or credentials are missing or refused: E_NO_PROJECT: \
         jira.project_key is not configured — a Jira create needs a project \
         key\n"
    );
    Ok(())
}

#[test]
fn a_linear_tracker_without_a_team_reports_the_missing_catalogue(
) -> Result<(), TestError> {
    let (code, stderr) = push_only_preview_stderr(
        "---\nwork:\n  integration: linear\nlinear:\n  pull:\n    \
         additional_teams: [core, ops]\n    filters:\n      label: [bug]\n      \
         state: [open]\n    max_items: 3\n    max_pages:\n      \
         discovery: 2\n  push:\n    max_items: unlimited\n---\n",
        LINEAR_TOKEN,
    )?;

    assert_eq!(code, Some(74), "{stderr}");
    assert_eq!(
        stderr,
        "work.integration names 'linear', which is wired but not usable — \
         its configuration or credentials are missing or refused: \
         E_CREATE_NO_CATALOGUE: no Linear team configured — set \
         linear.team_id, or run /init-linear to write catalogue.json\n"
    );
    Ok(())
}

const UNSYNCED_ITEM: &str = "---\ntype: \"work-item\"\nid: \"0001\"\n\
    title: \"Existing\"\ndate: \"2026-01-01T00:00:00Z\"\nauthor: \"Fixture\"\n\
    status: \"draft\"\nkind: \"task\"\npriority: \"medium\"\n\
    last_updated: \"2026-01-01T00:00:00Z\"\nlast_updated_by: \"Fixture\"\n\
    schema_version: 1\n---\n\n# 0001: Existing\n";

fn commit_everything(dir: &Path) -> Result<(), TestError> {
    for args in [
        &["add", "--all"][..],
        &[
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@example.com",
            "commit",
            "--quiet",
            "--message",
            "fixture",
        ],
    ] {
        let status =
            Command::new("git").args(args).current_dir(dir).status()?;
        assert!(status.success(), "git {args:?} failed");
    }
    Ok(())
}

struct RefusingProxy {
    url: String,
    contacts: Arc<AtomicUsize>,
}

impl RefusingProxy {
    fn start() -> Result<Self, TestError> {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let url = format!("http://{}", listener.local_addr()?);
        let contacts = Arc::new(AtomicUsize::new(0));
        let counter = Arc::clone(&contacts);
        std::thread::spawn(move || {
            for connection in listener.incoming() {
                counter.fetch_add(1, Ordering::SeqCst);
                drop(connection);
            }
        });
        Ok(Self { url, contacts })
    }

    fn contacts(&self) -> usize {
        self.contacts.load(Ordering::SeqCst)
    }
}

#[test]
fn a_push_only_preview_plans_an_unsynced_item_without_reaching_the_tracker(
) -> Result<(), TestError> {
    let repo = configured_repo(
        "---\nwork:\n  integration: jira\njira:\n  site: acme\n  \
         email: fixture@example.com\n  project_key: ENG\n---\n",
    )?;
    fs::write(
        repo.path().join("meta/work/0001-existing.md"),
        UNSYNCED_ITEM,
    )?;
    commit_everything(repo.path())?;
    let proxy = RefusingProxy::start()?;

    let output = run_with(
        repo.path(),
        &["--preview", "--push-only"],
        &[
            JIRA_TOKEN,
            ("HTTPS_PROXY", &proxy.url),
            ("https_proxy", &proxy.url),
            ("ALL_PROXY", &proxy.url),
            ("NO_PROXY", ""),
            ("no_proxy", ""),
        ],
    )?;

    let stderr = String::from_utf8(output.stderr)?;
    assert_eq!(output.status.code(), Some(0), "{stderr}");
    assert_eq!(
        String::from_utf8(output.stdout)?,
        "0001\tcreate-from-local\tunsynced\t-\n\
         0001\tnoop\tunsynced\t-\n\
         #\tdiscovery\tskipped\tpush-only\n"
    );
    assert_eq!(proxy.contacts(), 0, "the preview reached the network");
    Ok(())
}

#[test]
fn push_only_and_pull_only_together_is_a_usage_error() -> Result<(), TestError>
{
    let repo = scratch_repo(Some("jira"))?;
    let output = run(repo.path(), &["--push-only", "--pull-only"])?;
    assert_eq!(output.status.code(), Some(2));
    Ok(())
}

#[test]
fn a_malformed_resolve_order_is_a_usage_error() -> Result<(), TestError> {
    let repo = scratch_repo(Some("jira"))?;
    let output = run(repo.path(), &["--resolve", "no-equals-sign"])?;
    assert_eq!(output.status.code(), Some(2));
    Ok(())
}

#[test]
fn a_duplicate_resolve_order_for_one_id_is_a_usage_error(
) -> Result<(), TestError> {
    let repo = scratch_repo(Some("jira"))?;
    let output = run(
        repo.path(),
        &["--resolve", "0001=remote", "--resolve", "0001=local"],
    )?;
    assert_eq!(output.status.code(), Some(2));
    Ok(())
}

#[test]
fn stdin_closed_never_blocks_and_never_reads_it() -> Result<(), TestError> {
    let repo = scratch_repo(Some("jira"))?;
    let output = run(repo.path(), &[])?;
    // The process completing at all is the assertion: a run that hung on
    // stdin would hang this test.
    assert_eq!(output.status.code(), Some(74));
    Ok(())
}

#[test]
fn the_scrubbed_set_is_derived_from_the_token_ladders() -> Result<(), TestError>
{
    let scrubbed = common::provider_env_vars();
    for keys in [
        jira_client::auth::token_keys()?,
        linear_client::auth::token_keys()?,
    ] {
        for name in keys
            .plaintext
            .overrides
            .iter()
            .chain(keys.command.descriptor().overrides)
        {
            assert!(scrubbed.contains(&(*name).to_owned()), "{scrubbed:?}");
        }
    }
    assert_eq!(scrubbed.len(), 4, "two token env names per provider");
    Ok(())
}

#[test]
fn sync_help_names_every_exit_code() -> Result<(), TestError> {
    let repo = scratch_repo(None)?;
    let output = Command::new(env!("CARGO_BIN_EXE_accelerator-work"))
        .args(["sync", "--help"])
        .current_dir(repo.path())
        .output()?;
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout)?;
    for code in [
        "0", "1", "2", "3", "4", "5", "6", "70", "71", "72", "73", "74",
    ] {
        assert!(
            stdout.contains(code),
            "help text is missing exit code {code}: {stdout}"
        );
    }
    assert!(
        stdout.contains("matched no local id"),
        "help must name the target no-match (exit 3) cause: {stdout}"
    );
    assert!(
        stdout.contains("outside the work directory"),
        "help must name the out-of-directory (exit 6) cause: {stdout}"
    );
    Ok(())
}
