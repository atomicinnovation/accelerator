"""The signed attestation document the launcher's hit path verifies.

The manifest's inline signature is over the archive *file's bytes*, and the
launcher deletes the archive after extraction — so that signature has nothing
left on disk to verify against. The attestation is the separate, small document
the hit path actually checks: it binds artifact identity, platform and content,
so a repointed pointer cannot substitute another artifact or platform, and it
carries the ``.files`` table's digest so the table stays anchored after the
archive is discarded.

It deliberately carries neither the plugin release version (unknowable in the
job that assembles, which runs upstream of the version bump, and one archive set
serves two cuts) nor the launcher's layout version (consumer-owned policy a
signed copy could never let the launcher rewrite). Everything in it is knowable
at assembly time.

The field set and order match the launcher's ``Attestation`` reader exactly; a
drift test pins them together.
"""

from __future__ import annotations

import json

from tasks.vendor.archive import ArchiveStats

ATTESTATION_FORMAT_VERSION = 1


def build_attestation(
    artifact: str, platform: str, stats: ArchiveStats
) -> bytes:
    """Render the attestation document for one artifact on one platform.

    Emitted with sorted keys and a trailing newline, so the bytes are stable and
    the document the publishing job signs is byte-identical to the one assembly
    produced.
    """
    document = {
        "attestation_format_version": ATTESTATION_FORMAT_VERSION,
        "artifact": artifact,
        "platform": platform,
        "archive_sha256": stats.archive_sha256,
        "uncompressed_size": stats.uncompressed_size,
        "entry_count": stats.entry_count,
        "table_sha256": stats.table_sha256,
    }
    return (
        json.dumps(document, sort_keys=True, separators=(",", ":")) + "\n"
    ).encode("utf-8")
