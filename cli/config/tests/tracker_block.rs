//! Pins the structural validation of `<tracker>.pull` and `<tracker>.push`
//! blocks against the catalogue's description of them.

use config::catalogue::TRACKERS;
use config::tracker_block::{
    BlockError, BlockFault, BlockName, Field, FieldKind, TrackerBlock,
    TrackerCatalogue,
};
use config::{Level, Scalar, Value};

fn scalar(text: &str) -> Value {
    Value::Scalar(Scalar::String(text.to_owned()))
}

const fn int(value: i64) -> Value {
    Value::Scalar(Scalar::Int(value))
}

const fn flag(value: bool) -> Value {
    Value::Scalar(Scalar::Bool(value))
}

fn seq(items: &[&str]) -> Value {
    Value::Sequence(
        items
            .iter()
            .map(|item| Scalar::String((*item).to_owned()))
            .collect(),
    )
}

fn block(entries: Vec<(&str, Value)>) -> Value {
    Value::Mapping(
        entries
            .into_iter()
            .map(|(key, value)| (key.to_owned(), value))
            .collect(),
    )
}

fn owned(items: &[&str]) -> Vec<String> {
    items.iter().map(|item| (*item).to_owned()).collect()
}

fn pull(scope: &str, entries: Vec<(&str, Value)>) -> Result<(), BlockError> {
    TRACKERS.validate(scope, BlockName::Pull, &block(entries))
}

fn push(scope: &str, entries: Vec<(&str, Value)>) -> Result<(), BlockError> {
    TRACKERS.validate(scope, BlockName::Push, &block(entries))
}

const fn pull_fault(fault: BlockFault) -> Result<(), BlockError> {
    Err(BlockError {
        block: BlockName::Pull,
        fault,
    })
}

const fn push_fault(fault: BlockFault) -> Result<(), BlockError> {
    Err(BlockError {
        block: BlockName::Push,
        fault,
    })
}

const JIRA_PULL_KEYS: &[&str] = &[
    "additional_projects",
    "all_projects",
    "filters",
    "max_items",
    "max_pages",
];

fn unrecognised(key: &str, accepted: &[&str]) -> BlockFault {
    BlockFault::UnrecognisedKey {
        key: key.to_owned(),
        accepted: owned(accepted),
        hint: None,
    }
}

#[test]
fn a_well_formed_jira_pull_block_validates() {
    assert_eq!(
        pull(
            "jira",
            vec![
                ("additional_projects", seq(&["PP"])),
                (
                    "filters",
                    block(vec![
                        ("label", seq(&["a", "b"])),
                        ("state", scalar("open")),
                        ("assignee", scalar("me")),
                    ]),
                ),
                ("max_items", scalar("unlimited")),
                ("max_pages", int(50)),
            ],
        ),
        Ok(())
    );
}

#[test]
fn a_well_formed_linear_pull_block_validates() {
    assert_eq!(
        pull(
            "linear",
            vec![
                ("all_teams", flag(true)),
                ("filters", block(vec![("project", seq(&["Alpha"]))])),
                (
                    "max_pages",
                    block(vec![
                        ("default", int(9)),
                        ("discovery", int(20)),
                        ("keyed_read", scalar("unlimited")),
                    ]),
                ),
            ],
        ),
        Ok(())
    );
}

#[test]
fn a_non_mapping_block_is_not_a_mapping() {
    for value in [scalar("oops"), seq(&["a"])] {
        assert_eq!(
            TRACKERS.validate("jira", BlockName::Pull, &value),
            pull_fault(BlockFault::NotAMapping)
        );
        assert_eq!(
            TRACKERS.validate("linear", BlockName::Push, &value),
            push_fault(BlockFault::NotAMapping)
        );
    }
}

#[test]
fn a_non_mapping_filters_is_a_sub_block_fault() {
    assert_eq!(
        pull("jira", vec![("filters", scalar("oops"))]),
        pull_fault(BlockFault::SubBlockNotAMapping {
            key: "filters".to_owned()
        })
    );
}

#[test]
fn an_unsupported_filter_key_lists_the_trackers_accepted_set() {
    for (scope, accepted) in [
        ("jira", &["label", "state", "assignee"][..]),
        ("linear", &["label", "state", "assignee", "project"][..]),
    ] {
        assert_eq!(
            pull(
                scope,
                vec![("filters", block(vec![("colour", seq(&["red"]))]))]
            ),
            pull_fault(BlockFault::UnsupportedFilterKey {
                key: "colour".to_owned(),
                accepted: owned(accepted),
            }),
            "{scope}"
        );
    }
}

#[test]
fn a_reserved_grouping_key_is_a_nested_filter_fault() {
    for reserved in ["all", "any"] {
        assert_eq!(
            pull(
                "linear",
                vec![("filters", block(vec![(reserved, seq(&["x"]))]))]
            ),
            pull_fault(BlockFault::NestedFiltersUnsupported {
                key: reserved.to_owned(),
            })
        );
    }
}

#[test]
fn all_with_additional_is_mutually_exclusive() {
    assert_eq!(
        pull(
            "jira",
            vec![
                ("all_projects", flag(true)),
                ("additional_projects", seq(&["PP"])),
            ]
        ),
        pull_fault(BlockFault::MutuallyExclusive {
            first: "all_projects".to_owned(),
            second: "additional_projects".to_owned(),
        })
    );
}

#[test]
fn a_false_scope_flag_or_an_empty_list_does_not_engage_exclusivity() {
    assert_eq!(
        pull(
            "jira",
            vec![
                ("all_projects", flag(false)),
                ("additional_projects", seq(&["PP"])),
            ]
        ),
        Ok(())
    );
    assert_eq!(
        pull(
            "jira",
            vec![
                ("all_projects", flag(true)),
                ("additional_projects", seq(&[])),
            ]
        ),
        Ok(())
    );
}

#[test]
fn an_unknown_top_level_key_lists_the_blocks_fields() {
    assert_eq!(
        pull("jira", vec![("bogus", scalar("x"))]),
        pull_fault(unrecognised("bogus", JIRA_PULL_KEYS))
    );
}

#[test]
fn an_unknown_page_cap_sub_key_is_named_under_its_field() {
    assert_eq!(
        pull(
            "jira",
            vec![("max_pages", block(vec![("sideways", int(5))]))]
        ),
        pull_fault(unrecognised("max_pages.sideways", JIRA_PULL_KEYS))
    );
}

#[test]
fn a_wrong_tracker_noun_carries_a_hint_to_the_own_noun() {
    assert_eq!(
        pull("jira", vec![("all_teams", flag(true))]),
        pull_fault(BlockFault::UnrecognisedKey {
            key: "all_teams".to_owned(),
            accepted: owned(JIRA_PULL_KEYS),
            hint: Some(
                " `all_teams` is a Linear key; this integration is Jira — did \
                 you mean `all_projects`?"
                    .to_owned()
            ),
        })
    );
}

#[test]
fn max_items_accepts_zero_and_max_pages_refuses_it() {
    assert_eq!(pull("jira", vec![("max_items", int(0))]), Ok(()));
    assert_eq!(
        pull("jira", vec![("max_pages", int(0))]),
        pull_fault(BlockFault::BadCeiling {
            key: "max_pages".to_owned(),
            value: "0".to_owned(),
            allow_zero: false,
        })
    );
}

#[test]
fn ceilings_refuse_a_negative_a_float_and_text() {
    for (bad, rendered) in [
        (int(-1), "-1"),
        (Value::Scalar(Scalar::Float(1.5)), "1.5"),
        (scalar("lots"), "lots"),
    ] {
        assert_eq!(
            pull("jira", vec![("max_items", bad)]),
            pull_fault(BlockFault::BadCeiling {
                key: "max_items".to_owned(),
                value: rendered.to_owned(),
                allow_zero: true,
            })
        );
    }
}

#[test]
fn page_cap_overrides_are_named_under_their_field_and_default_is_bare() {
    assert_eq!(
        pull(
            "linear",
            vec![("max_pages", block(vec![("keyed_read", int(0))]))]
        ),
        pull_fault(BlockFault::BadCeiling {
            key: "max_pages.keyed_read".to_owned(),
            value: "0".to_owned(),
            allow_zero: false,
        })
    );
    assert_eq!(
        pull(
            "linear",
            vec![("max_pages", block(vec![("default", int(0))]))]
        ),
        pull_fault(BlockFault::BadCeiling {
            key: "max_pages".to_owned(),
            value: "0".to_owned(),
            allow_zero: false,
        })
    );
}

#[test]
fn page_caps_are_checked_default_then_overrides_in_declared_order() {
    assert_eq!(
        pull(
            "jira",
            vec![(
                "max_pages",
                block(vec![
                    ("keyed_read", int(0)),
                    ("discovery", int(0)),
                    ("default", int(0)),
                ])
            )]
        ),
        pull_fault(BlockFault::BadCeiling {
            key: "max_pages".to_owned(),
            value: "0".to_owned(),
            allow_zero: false,
        })
    );
}

#[test]
fn a_push_block_accepts_only_max_items() {
    assert_eq!(
        push("jira", vec![("max_items", scalar("unlimited"))]),
        Ok(())
    );
    assert_eq!(push("linear", vec![("max_items", int(0))]), Ok(()));
    assert_eq!(
        push("jira", vec![("filters", block(vec![]))]),
        push_fault(unrecognised("filters", &["max_items"]))
    );
    assert_eq!(
        push("linear", vec![("all_projects", flag(true))]),
        push_fault(unrecognised("all_projects", &["max_items"]))
    );
    assert_eq!(
        push("jira", vec![("max_items", scalar("2.5"))]),
        push_fault(BlockFault::BadCeiling {
            key: "max_items".to_owned(),
            value: "2.5".to_owned(),
            allow_zero: true,
        })
    );
}

#[test]
fn a_scope_with_no_such_block_has_nothing_to_violate() {
    assert!(TRACKERS.block("trello", BlockName::Pull).is_none());
    assert_eq!(
        TRACKERS.validate("trello", BlockName::Pull, &scalar("oops")),
        Ok(())
    );
}

#[test]
fn validation_reports_the_first_fault_in_stage_order() {
    let every_fault = |filters: Value| {
        vec![
            ("max_items", scalar("lots")),
            ("filters", filters),
            ("all_projects", flag(true)),
            ("additional_projects", seq(&["PP"])),
            ("bogus", scalar("x")),
            ("max_pages", block(vec![("sideways", int(1))])),
            ("additional_teams", seq(&["core"])),
        ]
    };
    let filters_with_faults =
        || block(vec![("colour", seq(&["red"])), ("any", seq(&["x"]))]);

    let mut entries = every_fault(scalar("oops"));
    assert!(matches!(
        pull("jira", entries.clone()),
        Err(BlockError {
            fault: BlockFault::SubBlockNotAMapping { .. },
            ..
        })
    ));

    entries = every_fault(filters_with_faults());
    assert!(matches!(
        pull("jira", entries.clone()),
        Err(BlockError { fault: BlockFault::UnrecognisedKey { ref key, hint: Some(_), .. }, .. })
            if key == "additional_teams"
    ));

    entries.retain(|(key, _)| *key != "additional_teams");
    assert_eq!(
        pull("jira", entries.clone()),
        pull_fault(unrecognised("bogus", JIRA_PULL_KEYS))
    );

    entries.retain(|(key, _)| *key != "bogus");
    assert_eq!(
        pull("jira", entries.clone()),
        pull_fault(unrecognised("max_pages.sideways", JIRA_PULL_KEYS))
    );

    entries.retain(|(key, _)| *key != "max_pages");
    assert!(matches!(
        pull("jira", entries.clone()),
        Err(BlockError {
            fault: BlockFault::MutuallyExclusive { .. },
            ..
        })
    ));

    entries.retain(|(key, _)| *key != "all_projects");
    assert_eq!(
        pull("jira", entries.clone()),
        pull_fault(BlockFault::UnsupportedFilterKey {
            key: "colour".to_owned(),
            accepted: owned(&["label", "state", "assignee"]),
        })
    );

    entries.retain(|(key, _)| *key != "filters");
    assert!(matches!(
        pull("jira", entries),
        Err(BlockError {
            fault: BlockFault::BadCeiling { .. },
            ..
        })
    ));
}

#[test]
fn within_a_stage_the_fault_written_first_is_reported() {
    assert_eq!(
        pull(
            "jira",
            vec![
                ("max_pages", block(vec![("nope", int(1))])),
                ("bogus", int(1)),
            ]
        ),
        pull_fault(unrecognised("max_pages.nope", JIRA_PULL_KEYS))
    );
    assert_eq!(
        pull(
            "jira",
            vec![(
                "filters",
                block(vec![("any", seq(&["x"])), ("colour", seq(&["red"]))])
            )]
        ),
        pull_fault(BlockFault::NestedFiltersUnsupported {
            key: "any".to_owned(),
        })
    );
}

fn assert_renders(cases: Vec<(BlockError, String)>) {
    for (error, expected) in cases {
        assert_eq!(error.detail(Level::Personal), expected);
    }
}

const fn pull_error(fault: BlockFault) -> BlockError {
    BlockError {
        block: BlockName::Pull,
        fault,
    }
}

const fn push_error(fault: BlockFault) -> BlockError {
    BlockError {
        block: BlockName::Push,
        fault,
    }
}

#[test]
fn every_pull_refusal_renders_todays_operator_text() {
    let fix_pull = "Fix the `pull` block in .accelerator/config.local.md.";
    assert_renders(vec![
        (
            pull_error(BlockFault::NotAMapping),
            format!(
                "the `pull` value must be a block of settings, not a scalar \
                 or list. {fix_pull}"
            ),
        ),
        (
            pull_error(BlockFault::SubBlockNotAMapping {
                key: "filters".to_owned(),
            }),
            format!(
                "the pull `filters` value must be a block of settings. \
                 {fix_pull}"
            ),
        ),
        (
            pull_error(BlockFault::UnsupportedFilterKey {
                key: "colour".to_owned(),
                accepted: owned(&["label", "state"]),
            }),
            format!(
                "the pull filter key `colour` is not supported (accepted: \
                 label, state). {fix_pull}"
            ),
        ),
        (
            pull_error(BlockFault::NestedFiltersUnsupported {
                key: "any".to_owned(),
            }),
            format!(
                "the pull filter key `any` is reserved: nested filters not \
                 yet supported. {fix_pull}"
            ),
        ),
        (
            pull_error(BlockFault::BadCeiling {
                key: "max_items".to_owned(),
                value: "lots".to_owned(),
                allow_zero: true,
            }),
            format!(
                "pull `max_items` must be a non-negative integer (0 refuses \
                 all) or `unlimited` (got `lots`). {fix_pull}"
            ),
        ),
        (
            pull_error(BlockFault::BadCeiling {
                key: "max_pages".to_owned(),
                value: "0".to_owned(),
                allow_zero: false,
            }),
            format!(
                "pull `max_pages` must be a positive integer or `unlimited` \
                 (got `0`). {fix_pull}"
            ),
        ),
        (
            pull_error(BlockFault::MutuallyExclusive {
                first: "all_teams".to_owned(),
                second: "additional_teams".to_owned(),
            }),
            format!(
                "a pull block sets both `all_*` and `additional_*` — remove \
                 one of `all_*`/`additional_*`. {fix_pull}"
            ),
        ),
        (
            pull_error(BlockFault::UnrecognisedKey {
                key: "all_teams".to_owned(),
                accepted: owned(&["all_projects", "filters"]),
                hint: Some(" try `all_projects`.".to_owned()),
            }),
            format!(
                "the pull block key `all_teams` is not recognised (accepted: \
                 all_projects, filters). try `all_projects`. {fix_pull}"
            ),
        ),
    ]);
}

#[test]
fn every_push_refusal_renders_todays_operator_text() {
    let fix_push = "Fix the `push` block in .accelerator/config.local.md.";
    assert_renders(vec![
        (
            push_error(BlockFault::NotAMapping),
            format!(
                "the `push` value must be a block of settings, not a scalar \
                 or list. {fix_push}"
            ),
        ),
        (
            push_error(BlockFault::BadCeiling {
                key: "max_items".to_owned(),
                value: "2.5".to_owned(),
                allow_zero: true,
            }),
            format!(
                "push `max_items` must be a non-negative integer (0 refuses \
                 all) or `unlimited` (got `2.5`). {fix_push}"
            ),
        ),
        (
            push_error(unrecognised("filters", &["max_items"])),
            format!(
                "the push block key `filters` is not recognised (accepted: \
                 max_items). {fix_push}"
            ),
        ),
    ]);
}

#[test]
fn the_detail_names_the_resolving_config_file() {
    let error = BlockError {
        block: BlockName::Pull,
        fault: BlockFault::NotAMapping,
    };
    assert!(error
        .detail(Level::Team)
        .ends_with(".accelerator/config.md."));
}

#[test]
fn every_block_pairs_one_entity_list_with_one_scope_flag_or_neither() {
    for tracker_block in TRACKERS.0 {
        let count = |wanted: FieldKind| {
            tracker_block
                .fields
                .iter()
                .filter(|field| field.kind == wanted)
                .count()
        };
        let lists = count(FieldKind::EntityList);
        let flags = count(FieldKind::ScopeFlag);
        assert!(
            (lists, flags) == (1, 1) || (lists, flags) == (0, 0),
            "{}.{}",
            tracker_block.scope,
            tracker_block.name
        );
    }
}

#[test]
fn every_pull_block_carries_a_scope_pair() {
    for tracker_block in TRACKERS.0 {
        if tracker_block.name == BlockName::Pull {
            assert!(
                tracker_block
                    .fields
                    .iter()
                    .any(|field| field.kind == FieldKind::EntityList),
                "{}",
                tracker_block.scope
            );
        }
    }
}

#[test]
fn the_catalogue_describes_pull_and_push_for_jira_and_linear() {
    let described: Vec<(&str, BlockName)> = TRACKERS
        .0
        .iter()
        .map(|tracker_block| (tracker_block.scope, tracker_block.name))
        .collect();
    assert_eq!(
        described,
        [
            ("jira", BlockName::Pull),
            ("linear", BlockName::Pull),
            ("jira", BlockName::Push),
            ("linear", BlockName::Push),
        ]
    );
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
        fields: &[Field {
            name: "max_comments",
            kind: FieldKind::Ceiling { allow_zero: false },
        }],
        mutually_exclusive: &[],
    },
];

const EXTENDED_CATALOGUE: TrackerCatalogue = TrackerCatalogue(EXTENDED);

#[test]
fn an_extension_field_validates_when_set_well() {
    assert_eq!(
        EXTENDED_CATALOGUE.validate(
            "acme",
            BlockName::Pull,
            &block(vec![(
                "labels_by_board",
                block(vec![("label", seq(&["x"]))])
            )]),
        ),
        Ok(())
    );
    assert_eq!(
        EXTENDED_CATALOGUE.validate(
            "acme",
            BlockName::Push,
            &block(vec![("max_comments", int(3))]),
        ),
        Ok(())
    );
}

#[test]
fn an_extension_field_is_structurally_invalid_when_set_badly() {
    assert_eq!(
        EXTENDED_CATALOGUE.validate(
            "acme",
            BlockName::Pull,
            &block(vec![("labels_by_board", scalar("oops"))]),
        ),
        pull_fault(BlockFault::SubBlockNotAMapping {
            key: "labels_by_board".to_owned()
        })
    );
    assert_eq!(
        EXTENDED_CATALOGUE.validate(
            "acme",
            BlockName::Push,
            &block(vec![("max_comments", int(0))]),
        ),
        push_fault(BlockFault::BadCeiling {
            key: "max_comments".to_owned(),
            value: "0".to_owned(),
            allow_zero: false,
        })
    );
}

#[test]
fn a_block_looks_up_by_scope_and_name() {
    let found = EXTENDED_CATALOGUE.block("acme", BlockName::Push);
    assert_eq!(found.map(|tracker_block| tracker_block.label), Some("Acme"));
    assert!(EXTENDED_CATALOGUE.block("jira", BlockName::Push).is_none());
}
