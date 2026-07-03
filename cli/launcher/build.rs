//! `fail_on_error()` is intentionally not called: vergen degrades a git-less or
//! shallow build to a placeholder rather than failing to compile.

use std::error::Error;

use vergen_gitcl::{BuildBuilder, CargoBuilder, Emitter, GitclBuilder};

const SHORT_SHA: bool = false;

fn main() -> Result<(), Box<dyn Error>> {
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
