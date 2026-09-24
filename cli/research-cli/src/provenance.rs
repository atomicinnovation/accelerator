//! Whether a file is tracked by the project's version control, as the
//! credential ladder asks it.

use std::path::Path;
use std::path::PathBuf;

use config::credentials::Provenance;
use vcs::VcsKind;
use vcs::VcsProbe as _;
use vcs_adapters::library::InProcessProbe;

pub struct VcsProvenance {
    root: PathBuf,
    kind: VcsKind,
}

impl VcsProvenance {
    pub fn discovered(root: PathBuf) -> Self {
        let kind = InProcessProbe.kind(&root);
        Self { root, kind }
    }
}

impl Provenance for VcsProvenance {
    fn is_tracked(&self, path: &Path) -> bool {
        let Some(relpath) =
            path.strip_prefix(&self.root).ok().and_then(Path::to_str)
        else {
            return false;
        };
        InProcessProbe
            .is_tracked(&self.root, relpath, self.kind)
            .unwrap_or(false)
    }
}
