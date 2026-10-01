//! `SyncedTeams`: the candidate teams a corpus's Linear identifiers name.

#![allow(clippy::expect_used, clippy::panic)]

use linear_client::healing::SyncedTeams;
use tracker::ExternalId;

fn ids(values: &[&str]) -> Vec<ExternalId> {
    values
        .iter()
        .map(|value| ExternalId::new((*value).to_owned()))
        .collect()
}

fn prefixes(synced: &SyncedTeams) -> Vec<(String, Vec<String>)> {
    synced
        .prefixes()
        .map(|(prefix, identifiers)| (prefix.to_owned(), identifiers.to_vec()))
        .collect()
}

#[test]
fn each_prefix_keeps_its_identifiers_in_sort_order() {
    let corpus = ids(&["PP-10", "ENG-3", "PP-2", "PP-869", "ENG-1"]);

    let synced = SyncedTeams::derive(&corpus);

    assert_eq!(
        prefixes(&synced),
        vec![
            (
                "ENG".to_owned(),
                vec!["ENG-1".to_owned(), "ENG-3".to_owned()]
            ),
            (
                "PP".to_owned(),
                vec![
                    "PP-2".to_owned(),
                    "PP-10".to_owned(),
                    "PP-869".to_owned()
                ]
            ),
        ]
    );
}

#[test]
fn local_work_item_ids_are_never_read() {
    for (local, external) in [("0292", "PP-869"), ("PROJ-0042", "PP-869")] {
        let external_ids = ids(&[external]);

        let synced = SyncedTeams::derive(&external_ids);

        assert_eq!(
            prefixes(&synced),
            vec![("PP".to_owned(), vec!["PP-869".to_owned()])],
            "the local id {local} never reaches derive"
        );
    }
}

#[test]
fn an_external_id_without_a_team_prefix_is_ignored() {
    let synced = SyncedTeams::derive(&ids(&["12", "-5", "ENG-", "ENG-x1"]));

    assert!(prefixes(&synced).is_empty());
}
