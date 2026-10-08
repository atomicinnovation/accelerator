//! Pins that the pull and push parsers derive their fields from the catalogue
//! they are handed, so a field the catalogue adds reaches a parsed config.

use config::tracker_block::{
    BlockError, BlockFault, BlockName, Field, FieldKind, TrackerBlock,
    TrackerCatalogue,
};
use config::{Scalar, Value};
use tracker_support::pull::{self, PullConfig, PullConfigError};
use tracker_support::push::{self, PushConfig, PushConfigError};

fn scalar(text: &str) -> Value {
    Value::Scalar(Scalar::String(text.to_owned()))
}

fn block(entries: Vec<(&str, Value)>) -> Value {
    Value::Mapping(
        entries
            .into_iter()
            .map(|(key, value)| (key.to_owned(), value))
            .collect(),
    )
}

static EXTENDED: &[TrackerBlock] = &[
    TrackerBlock {
        scope: "acme",
        label: "Acme",
        name: BlockName::Pull,
        fields: &[
            Field {
                name: "additional_boards",
                kind: FieldKind::EntityList,
            },
            Field {
                name: "all_boards",
                kind: FieldKind::ScopeFlag,
            },
            Field {
                name: "max_items",
                kind: FieldKind::Ceiling { allow_zero: true },
            },
            Field {
                name: "labels_by_board",
                kind: FieldKind::Filters {
                    accepted: &["label"],
                    reserved: &[],
                },
            },
        ],
        mutually_exclusive: &[],
    },
    TrackerBlock {
        scope: "acme",
        label: "Acme",
        name: BlockName::Push,
        fields: &[
            Field {
                name: "max_items",
                kind: FieldKind::Ceiling { allow_zero: true },
            },
            Field {
                name: "comment_filters",
                kind: FieldKind::Filters {
                    accepted: &["author"],
                    reserved: &[],
                },
            },
        ],
        mutually_exclusive: &[],
    },
];

const CATALOGUE: TrackerCatalogue = TrackerCatalogue(EXTENDED);

fn labels() -> Value {
    block(vec![("label", scalar("bug"))])
}

#[test]
fn a_pull_extension_field_set_well_lands_in_extensions_in_field_order() {
    let value = block(vec![
        ("labels_by_board", labels()),
        ("additional_boards", scalar("B1")),
        ("max_items", scalar("4")),
    ]);

    assert_eq!(pull::validate_with(CATALOGUE, "acme", &value), Ok(()));
    assert_eq!(
        pull::parse_with(CATALOGUE, "acme", &value),
        Ok(PullConfig {
            additional_entities: vec!["B1".to_owned()],
            max_items: Some(pull::CeilingToken("4".to_owned())),
            extensions: vec![("labels_by_board", labels())],
            ..PullConfig::default()
        })
    );
}

#[test]
fn a_pull_extension_field_set_badly_is_structurally_invalid() {
    let value = block(vec![("labels_by_board", scalar("oops"))]);
    let invalid = Err(PullConfigError::Structure(BlockError {
        block: BlockName::Pull,
        fault: BlockFault::SubBlockNotAMapping {
            key: "labels_by_board".to_owned(),
        },
    }));

    assert_eq!(pull::validate_with(CATALOGUE, "acme", &value), invalid);
    assert_eq!(
        pull::parse_with(CATALOGUE, "acme", &value).map(|_| ()),
        invalid
    );
}

#[test]
fn a_push_extension_field_set_well_lands_in_extensions() {
    let comments = block(vec![("author", scalar("me"))]);
    let value = block(vec![
        ("comment_filters", comments.clone()),
        ("max_items", scalar("2")),
    ]);

    assert_eq!(push::validate_with(CATALOGUE, "acme", &value), Ok(()));
    assert_eq!(
        push::parse_with(CATALOGUE, "acme", &value),
        Ok(PushConfig {
            max_items: Some("2".to_owned()),
            extensions: vec![("comment_filters", comments)],
        })
    );
}

#[test]
fn a_push_extension_field_set_badly_is_structurally_invalid() {
    let value = block(vec![("comment_filters", scalar("oops"))]);
    let invalid = Err(PushConfigError::Structure(BlockError {
        block: BlockName::Push,
        fault: BlockFault::SubBlockNotAMapping {
            key: "comment_filters".to_owned(),
        },
    }));

    assert_eq!(push::validate_with(CATALOGUE, "acme", &value), invalid);
    assert_eq!(
        push::parse_with(CATALOGUE, "acme", &value).map(|_| ()),
        invalid
    );
}
