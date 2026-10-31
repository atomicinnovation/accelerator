//! Whether text joined onto a directory stays inside it.

use std::path::Component;
use std::path::Path;

/// Whether `text` is exactly one normal path component: no separator, and
/// neither `.` nor `..`.
#[must_use]
pub fn is_one_path_component(text: &str) -> bool {
    let mut components = Path::new(text).components();
    !text.contains(['/', '\\'])
        && matches!(
            (components.next(), components.next()),
            (Some(Component::Normal(_)), None)
        )
}

/// Whether `text` is a relative path of `/`-separated normal components.
#[must_use]
pub fn is_relative_path_of_components(text: &str) -> bool {
    !text.is_empty() && text.split('/').all(is_one_path_component)
}

#[cfg(test)]
mod tests {
    use super::is_one_path_component;
    use super::is_relative_path_of_components;

    #[test]
    fn one_component_has_no_separator_and_is_not_a_dot_entry() {
        assert!(is_one_path_component("PP-0001"));
        for refused in ["", ".", "..", "a/b", "a\\b", "/x", "a/"] {
            assert!(!is_one_path_component(refused), "{refused:?}");
        }
    }

    #[test]
    fn a_relative_path_is_normal_components_joined_by_slashes() {
        assert!(is_relative_path_of_components("meta/work/0001-x.md"));
        for refused in ["", "../x", "/tmp/x", "a//b", "a/./b", "a/"] {
            assert!(!is_relative_path_of_components(refused), "{refused:?}");
        }
    }
}
