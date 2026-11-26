from tasks.shared.shared_fixtures.families import Family


def test_every_family_has_each_of_its_traits():
    assert all(
        family.handover_variable
        and family.flag
        and family.heading
        and family.noun
        for family in Family
    )


def test_only_the_stalled_migration_set_needs_an_executable_to_build():
    assert {family: dict(family.executables) for family in Family} == {
        Family.MATRIX: {},
        Family.STATUS_LOG: {},
        Family.STALLED_MIGRATION: {
            "ACCELERATOR_SHARED_FIXTURES_MIGRATE": "accelerator-migrate"
        },
    }
