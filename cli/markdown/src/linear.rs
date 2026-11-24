//! Markdown as Linear's API parser reads it.

mod top_level_domains;

pub use top_level_domains::is_top_level_domain;

#[cfg(test)]
mod tests;
