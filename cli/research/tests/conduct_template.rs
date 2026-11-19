use research::conduct::template::ConductTemplate;
use research::confinement::PermittedCommand;

#[test]
fn conduct_fills_a_finding_and_a_level_note() {
    let named: Vec<String> = ConductTemplate::ALL
        .iter()
        .map(ConductTemplate::name)
        .collect();
    assert_eq!(
        named,
        ["topic-research-finding", "topic-research-level-note"]
    );
}

#[test]
fn each_template_is_permitted_exactly_the_command_that_fetches_it() {
    for template in ConductTemplate::ALL {
        let expected = format!(
            "accelerator config template {} --kind {}",
            ConductTemplate::SET,
            template.kind()
        );
        assert_eq!(
            template.permitted_command(),
            PermittedCommand::Exactly(expected.leak()),
            "{template:?}"
        );
    }
}
