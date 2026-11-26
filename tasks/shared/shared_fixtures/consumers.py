"""Which nextest binaries use each shared family.

Kept per binary rather than per test, so renaming, adding or splitting a Rust
test never touches it; a filtered run selecting only a consumer's tests that
use no fixture builds the family anyway.
"""

from collections.abc import Mapping

from tasks.shared.nextest_listing import NextestListing
from tasks.shared.shared_fixtures.families import Family

_MATRIX = frozenset({Family.MATRIX})
_STATUS_LOG = frozenset({Family.STATUS_LOG})
_STALLED_MIGRATION = frozenset({Family.STALLED_MIGRATION})

CONSUMERS: Mapping[str, frozenset[Family]] = {
    "accelerator-migrate::guarded_resume": _STALLED_MIGRATION,
    "accelerator-vcs::status_log_goldens": _STATUS_LOG,
    "vcs-adapters::classify": _MATRIX,
    "vcs-adapters::conflicts": _STATUS_LOG,
    "vcs-adapters::queries": _MATRIX,
    "vcs-adapters::scrub": _MATRIX,
    "vcs-test-support::copy_fidelity": _STATUS_LOG,
    "vcs-test-support::matrix": _MATRIX,
    "vcs-test-support::matrix_private_build": _MATRIX,
    "vcs-test-support::status_log_parts": _STATUS_LOG,
    "vcs-test-support::status_log_states": _STATUS_LOG,
}


def consumers_of(family: Family) -> frozenset[str]:
    return frozenset(
        binary for binary, consumed in CONSUMERS.items() if family in consumed
    )


def families_consumed(listing: NextestListing) -> frozenset[Family]:
    return frozenset(
        family
        for binary in listing.binaries
        for family in CONSUMERS.get(binary, frozenset())
    )
