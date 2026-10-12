//! The body rules sync and promotion apply, over the tracker's dialect.

use remote_projection::split_projected;
use tracker::BodyDialect;
use tracker::Unrenderable;

/// The single owner of how a tracker's projected body relates to a local
/// work item's body.
pub struct BodyComparison<'a> {
    dialect: &'a dyn BodyDialect,
}

impl<'a> BodyComparison<'a> {
    #[must_use]
    pub fn new(dialect: &'a dyn BodyDialect) -> Self {
        Self { dialect }
    }

    /// The local body a pull of `projected` writes: the description below
    /// the projected title line, rendered through the tracker's dialect.
    ///
    /// # Errors
    ///
    /// [`Unrenderable`] when the description has no Markdown form.
    pub fn local_body(&self, projected: &str) -> Result<String, Unrenderable> {
        self.dialect
            .local_body(split_projected(projected).description)
    }
}
