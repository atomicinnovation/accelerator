//! The VCS-mode-aware overwrite guard's pure decision logic.
//!
//! Takes already-fetched VCS status text as input — the real `jj diff`/
//! `git status` shellout is the adapter's job. Not called from `work
//! update`'s control flow, which performs an unconditional atomic write.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VcsMode {
    Jj,
    Git,
    Indeterminate,
}

/// True iff `path_relative` is dirty under `mode`.
///
/// `status_text` is that VCS's already-fetched status output (`jj diff
/// --name-only` for `Jj`, `git status --porcelain -- <path>` for `Git`).
/// [`VcsMode::Indeterminate`] fails safe to dirty.
#[must_use]
pub fn is_dirty(mode: VcsMode, path_relative: &str, status_text: &str) -> bool {
    match mode {
        VcsMode::Jj => status_text.lines().any(|line| line == path_relative),
        VcsMode::Git => !status_text.trim().is_empty(),
        VcsMode::Indeterminate => true,
    }
}

#[cfg(test)]
mod tests {
    use super::{is_dirty, VcsMode};

    #[test]
    fn git_porcelain_non_empty_is_dirty() {
        assert!(is_dirty(
            VcsMode::Git,
            "meta/work/0001-x.md",
            " M meta/work/0001-x.md"
        ));
    }

    #[test]
    fn git_porcelain_empty_is_clean() {
        assert!(!is_dirty(VcsMode::Git, "meta/work/0001-x.md", ""));
    }

    #[test]
    fn git_untracked_marker_is_dirty() {
        assert!(is_dirty(
            VcsMode::Git,
            "meta/work/0001-x.md",
            "?? meta/work/0001-x.md"
        ));
    }

    #[test]
    fn jj_path_present_in_diff_is_dirty() {
        assert!(is_dirty(
            VcsMode::Jj,
            "meta/work/0001-x.md",
            "meta/work/0001-x.md"
        ));
    }

    #[test]
    fn jj_path_absent_from_diff_is_clean() {
        assert!(!is_dirty(
            VcsMode::Jj,
            "meta/work/0001-x.md",
            "meta/work/other.md"
        ));
    }

    #[test]
    fn indeterminate_mode_fails_safe_to_dirty() {
        assert!(is_dirty(VcsMode::Indeterminate, "meta/work/0001-x.md", ""));
    }
}
