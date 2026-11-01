//! The work-item ID reads typed linkage makes.

/// How linkage reads a work-item ID from a path and from body text.
pub trait LinkageIds {
    fn id_in_filename(&self, filename: &str) -> Option<String>;

    fn is_bare_id(&self, token: &str) -> bool;
}
