from pathlib import Path

import pytest

from tasks.shared.build_measurement.cache_states import BuildCacheState


@pytest.mark.parametrize(
    ("state", "primes", "touches", "deletions"),
    [
        (BuildCacheState.WARM_NO_OP, True, (), ()),
        (
            BuildCacheState.INCREMENTAL,
            True,
            (Path("cli/kernel/src/lib.rs"),),
            (),
        ),
        (
            BuildCacheState.COLD,
            False,
            (),
            (Path("cli/target"), Path("cli/visualiser/frontend/node_modules")),
        ),
    ],
)
def test_each_state_names_what_it_touches_and_deletes(
    state, primes, touches, deletions
):
    assert state.primes is primes
    assert state.touches == touches
    assert state.deletions == deletions


def test_the_states_are_recorded_by_their_flag_values():
    assert [str(state) for state in BuildCacheState] == [
        "warm-no-op",
        "incremental",
        "cold",
    ]
