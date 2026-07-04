//! Emits the `version` subcommand's build metadata via vergen, and copies the
//! single committed release public key into `OUT_DIR` for the launcher to embed.
//!
//! `fail_on_error()` is intentionally not called for vergen: it degrades a
//! git-less or shallow build to a placeholder rather than failing to compile.
//! The key copy, by contrast, fails loudly — a launcher that cannot embed its
//! trust root must not build.

use std::error::Error;
use std::path::Path;

use vergen_gitcl::{BuildBuilder, CargoBuilder, Emitter, GitclBuilder};

const SHORT_SHA: bool = false;

fn main() -> Result<(), Box<dyn Error>> {
    embed_release_public_key()?;

    let build = BuildBuilder::default().build_timestamp(true).build()?;
    let cargo = CargoBuilder::default().target_triple(true).build()?;
    let gitcl = GitclBuilder::default().sha(SHORT_SHA).build()?;
    Emitter::default()
        .add_instructions(&build)?
        .add_instructions(&cargo)?
        .add_instructions(&gitcl)?
        .emit()?;
    Ok(())
}

/// Copy the single committed release public key (`keys/accelerator-release.pub`
/// at the repo root — the same file the bootstrap ships) into `OUT_DIR` so the
/// launcher can `include_str!` it. Running in `build.rs` (not a mise step) means
/// every `cargo` invocation — including rust-analyzer and a bare `cargo test` —
/// has the key, and it stays a single source of truth with no coherence check:
/// the launcher-embedded key and the bootstrap-shipped key are the same bytes.
fn embed_release_public_key() -> Result<(), Box<dyn Error>> {
    // CARGO_MANIFEST_DIR is cli/launcher; the committed key is at repo/keys.
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")?;
    let source =
        Path::new(&manifest_dir).join("../../keys/accelerator-release.pub");
    let out_dir = std::env::var("OUT_DIR")?;
    let destination = Path::new(&out_dir).join("release.pub");

    let key = std::fs::read(&source).map_err(|error| {
        format!(
            "cannot read the release public key at {}: {error}",
            source.display()
        )
    })?;
    std::fs::write(&destination, key)?;
    // Rebuild if the committed key changes (e.g. a rotation).
    println!("cargo:rerun-if-changed={}", source.display());
    Ok(())
}
