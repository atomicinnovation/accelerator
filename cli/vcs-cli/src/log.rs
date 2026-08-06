//! `vcs log`: `jj log --limit 5`, or `git log --oneline -5` in a git (or
//! repository-less) checkout.

use std::path::Path;

use vcs::RepoRoot as _;
use vcs::VcsKind;
use vcs::VcsProbe as _;
use vcs_adapters::library::InProcessProbe;
use vcs_adapters::subprocess;

/// Never fails — see [`vcs_adapters::subprocess::log`].
#[must_use]
pub fn run(start: &Path) -> String {
    let probe = InProcessProbe;
    let root = probe.discover(start);
    let kind = root
        .as_deref()
        .map_or(VcsKind::Git, |root| probe.kind(root));
    let dir = root.as_deref().unwrap_or(start);
    subprocess::log(dir, kind)
}
