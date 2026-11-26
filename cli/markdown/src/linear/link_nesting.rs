//! Whether an event falls inside a link or an image, whose text Linear
//! neither links nor reads as a list.

use document::markdown::{MarkdownEvent, MarkdownTag};

#[derive(Debug, Default)]
pub(super) struct LinkNesting {
    depth: usize,
}

impl LinkNesting {
    pub(super) const fn observe(&mut self, event: &MarkdownEvent) {
        match event {
            MarkdownEvent::Start(
                MarkdownTag::Link { .. } | MarkdownTag::Image { .. },
            ) => self.depth += 1,
            MarkdownEvent::End(
                MarkdownTag::Link { .. } | MarkdownTag::Image { .. },
            ) => self.depth = self.depth.saturating_sub(1),
            _ => {}
        }
    }

    pub(super) const fn is_outside(&self) -> bool {
        self.depth == 0
    }
}
