//! The work-item ID reads typed linkage makes.

/// How linkage reads a work-item ID from a path and from body text.
pub trait LinkageIds {
    fn id_in_filename(&self, filename: &str) -> Option<String>;

    /// The bare work-item IDs a `## Dependencies` line names, in order.
    fn bare_ids_in(&self, line: &str) -> Vec<String>;
}

#[cfg(test)]
pub(crate) mod test_ids {
    use super::LinkageIds;

    /// A port that knows a fixed set of IDs, so linkage is tested against
    /// the port rather than a copy of any shipped reading: a filename names
    /// one when it starts with it before a `-`, and a line names each that
    /// stands as a whole word.
    pub struct KnownIds(pub &'static [&'static str]);

    pub const KNOWN: KnownIds = KnownIds(&[
        "0002", "0003", "0026", "0030", "0034", "0042", "0055", "0061", "0063",
        "0065",
    ]);

    impl LinkageIds for KnownIds {
        fn id_in_filename(&self, filename: &str) -> Option<String> {
            let (head, _) = filename.split_once('-')?;
            self.0.contains(&head).then(|| head.to_owned())
        }

        fn bare_ids_in(&self, line: &str) -> Vec<String> {
            line.split(|c: char| !c.is_ascii_alphanumeric())
                .filter(|word| self.0.contains(word))
                .map(str::to_owned)
                .collect()
        }
    }
}
