use cli_test_support::source_scan::{
    built_in_doc_type_references, production_code,
};

#[test]
fn a_qualified_built_in_constant_is_found() {
    let source = "fn rank(kind: DocTypeKey) -> bool {\n    \
                  kind == corpus::DocTypeKey::PLANS\n}\n";
    assert_eq!(
        built_in_doc_type_references(source),
        ["2: kind == corpus::DocTypeKey::PLANS"]
    );
}

#[test]
fn a_constant_named_through_an_alias_of_the_key_is_found() {
    let source = "use corpus::DocTypeKey as Kind;\n\
                  fn plans() -> Kind { Kind::PLANS }\n";
    assert_eq!(
        built_in_doc_type_references(source),
        ["2: fn plans() -> Kind { Kind::PLANS }"]
    );
}

#[test]
fn a_constant_named_through_a_type_alias_of_the_key_is_found() {
    let source = "type Kind = corpus::DocTypeKey;\n\
                  const NOTES: Kind = Kind::NOTES;\n";
    assert_eq!(
        built_in_doc_type_references(source),
        ["2: const NOTES: Kind = Kind::NOTES;"]
    );
}

#[test]
fn an_import_from_the_built_in_declarations_is_found() {
    let source = "use crate::doc_type::built_in::IN_CATALOGUE_ORDER;\n";
    assert_eq!(
        built_in_doc_type_references(source),
        ["1: use crate::doc_type::built_in::IN_CATALOGUE_ORDER;"]
    );
}

#[test]
fn reading_a_key_s_declared_facts_is_not_a_reference() {
    let source = "fn shown(kind: DocTypeKey) -> bool {\n    \
                  !kind.is_virtual() && DocTypeKey::declared(&X).in_kanban()\n}\n";
    assert!(built_in_doc_type_references(source).is_empty());
}

#[test]
fn a_constant_inside_a_test_gated_item_is_ignored() {
    let source = "#[cfg(test)]\n\
                  #[allow(clippy::expect_used)]\n\
                  mod tests {\n    \
                  fn plans() { let _ = DocTypeKey::PLANS; }\n\
                  }\n\
                  #[cfg(test)]\n\
                  fn notes() -> DocTypeKey { DocTypeKey::NOTES }\n";
    assert!(built_in_doc_type_references(source).is_empty());
}

#[test]
fn a_constant_after_a_test_gated_module_is_found() {
    let source = "#[cfg(test)]\n\
                  mod testing;\n\
                  #[cfg(test)]\n\
                  mod tests {\n    \
                  fn brace_in_a_string() { let _ = \"}\"; }\n\
                  }\n\
                  fn plans() -> DocTypeKey { DocTypeKey::PLANS }\n";
    assert_eq!(
        built_in_doc_type_references(source),
        ["7: fn plans() -> DocTypeKey { DocTypeKey::PLANS }"]
    );
}

#[test]
fn a_test_gated_field_ends_at_its_comma() {
    let source = "struct Indexer {\n    \
                  #[cfg(test)]\n    \
                  hook: Mutex<Option<Hook>>,\n    \
                  kind: Kind,\n\
                  }\n";
    let production = production_code(source);
    assert!(production.contains("kind: Kind"), "{production}");
    assert!(!production.contains("hook"), "{production}");
    assert_eq!(production.lines().count(), source.lines().count());
}

#[test]
fn a_constant_inside_a_comment_or_literal_is_not_a_reference() {
    let source = "// DocTypeKey::PLANS\n\
                  const NAME: &str = \"DocTypeKey::PLANS\";\n\
                  const RAW: &str = r#\"doc_type::built_in\"#;\n";
    assert!(built_in_doc_type_references(source).is_empty());
}
