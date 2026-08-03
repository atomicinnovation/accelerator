"""Tests for the `lint:dispatch-coherence:check` leaf.

The guard's own behaviour lives in
`tests/unit/tasks/shared/test_dispatch_coherence.py`; this file proves only
that the task surfaces a problem as a failure. Without it an inverted condition
or a `print` in place of the `raise` leaves the PR-time gate green forever.
"""

from unittest.mock import MagicMock

import pytest
from invoke import Context, Exit

from tasks.lint import dispatch_coherence


@pytest.fixture
def ctx():
    return MagicMock(spec=Context)


def test_raises_on_problems(ctx, mocker) -> None:
    mocker.patch.object(
        dispatch_coherence, "violations", return_value=["frob: unbound"]
    )
    with pytest.raises(Exit, match="frob: unbound"):
        dispatch_coherence.check(ctx)


def test_does_not_raise_when_clean(ctx, mocker) -> None:
    mocker.patch.object(dispatch_coherence, "violations", return_value=[])
    dispatch_coherence.check(ctx)
