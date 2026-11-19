"""Which nextest binaries use each shared family.

Kept per binary rather than per test, so renaming, adding or splitting a Rust
test never touches it; a filtered run selecting only a consumer's tests that
use no fixture builds the family anyway.
"""

from collections.abc import Mapping

from tasks.shared.shared_fixtures.families import Family

CONSUMERS: Mapping[str, Family] = {
    "vcs-adapters::classify": Family.MATRIX,
    "vcs-adapters::queries": Family.MATRIX,
    "vcs-adapters::scrub": Family.MATRIX,
    "vcs-test-support::matrix": Family.MATRIX,
}


def consumers_of(family: Family) -> frozenset[str]:
    return frozenset(
        binary for binary, consumed in CONSUMERS.items() if consumed is family
    )
