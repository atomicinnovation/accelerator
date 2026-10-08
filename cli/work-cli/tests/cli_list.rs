//! CLI-boundary tests for `work list`.

use std::fs;
use std::process::Command;
use std::process::Stdio;

mod common;

type TestError = Box<dyn std::error::Error>;

const SYNCED_ITEM: &str = "---\ntype: \"work-item\"\nid: \"0001\"\n\
    title: \"Existing\"\ndate: \"2026-01-01T00:00:00Z\"\nauthor: \"Fixture\"\n\
    status: \"draft\"\nkind: \"task\"\npriority: \"medium\"\n\
    external_id: \"ENG-1\"\nlast_updated: \"2026-01-01T00:00:00Z\"\n\
    last_updated_by: \"Fixture\"\nschema_version: 1\n---\n\n# 0001: Existing\n";

const STALE_BASELINE: &str = "{\"timestamp\": 1, \"items\": {\"0001\": \
    {\"remote_updated_at\": \"2026-06-01T00:00:00Z\", \"remote_hash\": \
    \"rh\", \"local_hash\": \
    \"0000000000000000000000000000000000000000000000000000000000000000\"}}}\n";

#[test]
fn an_uncredentialed_integration_shows_the_presence_label_despite_a_baseline(
) -> Result<(), TestError> {
    let dir = tempfile::Builder::new()
        .prefix("work-cli-list-")
        .tempdir()?;
    let status = Command::new("git")
        .args(["init", "-q"])
        .current_dir(dir.path())
        .status()?;
    assert!(status.success(), "git init failed");
    fs::create_dir_all(dir.path().join("meta/work"))?;
    fs::create_dir_all(
        dir.path().join(".accelerator/state/integrations/jira"),
    )?;
    fs::write(
        dir.path().join(".accelerator/config.md"),
        "---\nwork:\n  integration: jira\njira:\n  site: acme\n  \
         email: fixture@example.com\n  project_key: ENG\n---\n",
    )?;
    fs::write(dir.path().join("meta/work/0001-existing.md"), SYNCED_ITEM)?;
    fs::write(
        dir.path()
            .join(".accelerator/state/integrations/jira/last-sync.json"),
        STALE_BASELINE,
    )?;
    let mut command = Command::new(env!("CARGO_BIN_EXE_accelerator-work"));
    command
        .arg("list")
        .current_dir(dir.path())
        .env("ACCELERATOR_PLUGIN_ROOT", dir.path())
        .stdin(Stdio::null());
    common::scrub_provider_env(&mut command);

    let output = command.output()?;

    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        String::from_utf8(output.stdout)?,
        "All work items (1 total)\n\
         | ID | Title | Kind | Status | Priority | Sync |\n\
         | --- | --- | --- | --- | --- | --- |\n\
         | 0001 | Existing | task | draft | medium | 🟢 synced |\n"
    );
    Ok(())
}
