//! Pins agent-name resolution over a configured `agents:` section.

use config::catalogue::agent_name;
use config::{
    ConfigError, ConfigService, Level, Node, ReadConfigLevel, Scalar,
    WriteConfigLevel,
};

struct PersonalAgents(Vec<(&'static str, &'static str)>);

impl ReadConfigLevel for PersonalAgents {
    fn read(&self, level: Level) -> Result<Option<Node>, ConfigError> {
        if level == Level::Team {
            return Ok(None);
        }
        let agents = self
            .0
            .iter()
            .map(|(name, value)| {
                (
                    (*name).to_owned(),
                    Node::Scalar(Scalar::String((*value).to_owned())),
                )
            })
            .collect();
        Ok(Some(Node::Mapping(
            std::iter::once(("agents".to_owned(), Node::Mapping(agents)))
                .collect(),
        )))
    }
}

struct NoWrites;

impl WriteConfigLevel for NoWrites {
    fn write(&self, _: Level, _: &Node) -> Result<(), ConfigError> {
        Ok(())
    }
}

const fn configured(
    agents: Vec<(&'static str, &'static str)>,
) -> ConfigService<PersonalAgents, NoWrites> {
    ConfigService::new(PersonalAgents(agents), NoWrites)
}

#[test]
fn an_override_is_returned_verbatim() -> Result<(), ConfigError> {
    let config = configured(vec![("researcher", "custom:researcher")]);
    assert_eq!(agent_name(&config, "researcher")?, "custom:researcher");
    Ok(())
}

#[test]
fn an_unset_agent_resolves_to_the_prefixed_default() -> Result<(), ConfigError>
{
    let config = configured(Vec::new());
    assert_eq!(agent_name(&config, "researcher")?, "accelerator:researcher");
    Ok(())
}

#[test]
fn an_explicit_empty_override_coalesces_to_the_prefixed_default(
) -> Result<(), ConfigError> {
    let config = configured(vec![("reviewer", "")]);
    assert_eq!(agent_name(&config, "reviewer")?, "accelerator:reviewer");
    Ok(())
}

#[test]
fn an_uncatalogued_name_resolves_to_the_prefixed_default(
) -> Result<(), ConfigError> {
    let config = configured(Vec::new());
    assert_eq!(
        agent_name(&config, "no-such-agent")?,
        "accelerator:no-such-agent"
    );
    Ok(())
}

#[test]
fn a_malformed_name_is_a_config_error() {
    let config = configured(Vec::new());
    assert!(agent_name(&config, "").is_err());
}
