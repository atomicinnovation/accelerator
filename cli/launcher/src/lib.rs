pub fn crate_name() -> &'static str {
    "launcher"
}

#[cfg(test)]
mod tests {
    use super::crate_name;

    #[test]
    fn reports_its_name() {
        assert_eq!(crate_name(), "launcher");
    }
}
