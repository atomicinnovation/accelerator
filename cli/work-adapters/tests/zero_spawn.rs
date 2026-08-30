//! Proves the crate's render paths reach no external binary at run time.
//!
//! The `work_adapters_is_zero_spawn` pup rule catches use-path `std::process`
//! imports at build time but is blind to an inline
//! `std::process::Command::new("diff")`. This harness closes that gap: it points
//! the test process's own `PATH` at a directory of tripwire executables (`diff`
//! plus `sh`/`bash`), each writing a marker when run, then drives `diff::render`
//! and `render_dossier` in-process and asserts the marker was never written.
//!
//! Unlike `corpus-adapters`' black-box `zero_spawn.rs`, there is nothing to
//! degrade here — `render` is pure text diffing — so no fixture `[[bin]]` or
//! output comparison is needed. The process-global `PATH` mutation is safe
//! because nextest runs each test in its own process. Like that precedent, the
//! tripwire catches only `PATH`-resolved bare-command spawns; an absolute-path
//! spawn would not fire it, matching the threat model of a re-introduced bare
//! `diff` shell-out.
#![cfg(feature = "bash-parity")]

use std::fs;
use std::os::unix::fs::PermissionsExt as _;
use std::path::Path;

use tracker::RemoteTimestamp;
use work::section_diff::SectionDiff;
use work_adapters::diff::render;
use work_adapters::sync::run::render_dossier;
use work_adapters::sync::run::ConflictDossier;
use work_adapters::sync::run::DossierRender;

type TestError = Box<dyn std::error::Error>;

fn write_tripwire(
    dir: &Path,
    name: &str,
    marker: &Path,
) -> Result<(), TestError> {
    let script = dir.join(name);
    let marker = marker.display();
    fs::write(&script, format!("#!/bin/sh\n: > \"{marker}\"\n"))?;
    fs::set_permissions(&script, fs::Permissions::from_mode(0o755))?;
    Ok(())
}

fn section(name: &str, local: &str, remote: &str) -> SectionDiff {
    SectionDiff {
        name: name.to_owned(),
        local: local.to_owned(),
        remote: remote.to_owned(),
    }
}

#[test]
fn the_render_paths_reach_no_external_binary() -> Result<(), TestError> {
    let stub = tempfile::Builder::new()
        .prefix("work-adapters-zero-spawn-")
        .tempdir()?;
    let marker = stub.path().join("spawned.marker");
    for name in ["diff", "sh", "bash"] {
        write_tripwire(stub.path(), name, &marker)?;
    }
    std::env::set_var("PATH", stub.path());

    let differing = section("Summary", "local\n", "remote\n");
    let _ = render(&differing);

    let dossier = ConflictDossier {
        id: "0001".to_owned(),
        title: "Title".to_owned(),
        local_modified: None,
        remote_updated: RemoteTimestamp::NotRead,
        sections: vec![differing],
        local_unreadable: false,
    };
    assert!(matches!(
        render_dossier(&dossier, &render),
        DossierRender::Renderable(_)
    ));

    assert!(
        !marker.exists(),
        "work-adapters spawned an external binary: the tripwire wrote its marker"
    );
    Ok(())
}
