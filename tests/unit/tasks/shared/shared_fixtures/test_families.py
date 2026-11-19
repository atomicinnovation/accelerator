from tasks.shared.shared_fixtures.families import Family


def test_every_family_has_each_of_its_traits():
    assert all(
        family.handover_variable
        and family.flag
        and family.heading
        and family.noun
        for family in Family
    )
