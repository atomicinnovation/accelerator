"""Raise this process's file-descriptor soft limit for link-heavy builds.

zig's linker opens every object file of a link at once, and a release link of
the cli workspace passes it several hundred. macOS's launchd default soft limit
is 256 descriptors, so a `cargo zigbuild` from a stock shell fails partway
through with `ProcessFdQuotaExceeded` (EMFILE) on an object it cannot open —
a failure that reads like a broken build but is purely an environment limit.

`setrlimit` is inherited across `fork`/`exec`, so raising it in the task process
covers cargo, rustc, and the zig wrapper beneath them.
"""

import resource
import sys

# Comfortably above the largest link in the workspace (~640 objects) with room
# for the descriptors cargo and rustc hold alongside it. Deliberately not
# "as high as possible": the soft limit is inherited by every child, and some
# tools size per-descriptor bookkeeping off it.
LINK_DESCRIPTOR_TARGET = 65_536


def descriptor_limit_raise_to(
    soft: int, hard: int, *, wanted: int = LINK_DESCRIPTOR_TARGET
) -> int | None:
    """Return the soft limit to set, or ``None`` if no raise applies.

    Pure over the ``getrlimit`` pair so the clamping is unit-testable without
    mutating the test runner's own limits. Never lowers an already-adequate
    limit, and never exceeds the hard limit — on Linux that is a finite ceiling
    an unprivileged process cannot cross, whereas macOS reports it as
    ``RLIM_INFINITY``.
    """
    if soft == resource.RLIM_INFINITY or soft >= wanted:
        return None
    target = wanted if hard == resource.RLIM_INFINITY else min(wanted, hard)
    return target if target > soft else None


def raise_descriptor_limit(
    *, wanted: int = LINK_DESCRIPTOR_TARGET
) -> int | None:
    """Raise the soft descriptor limit toward ``wanted``; report what was set.

    Best-effort by design: a build under an already-generous limit needs no
    raise, and one that cannot raise may still link fine (few enough objects).
    Failing here would turn a survivable condition into a hard stop, so a
    refused `setrlimit` warns and continues — the link itself fails loudly if
    the limit really was the constraint.
    """
    soft, hard = resource.getrlimit(resource.RLIMIT_NOFILE)
    target = descriptor_limit_raise_to(soft, hard, wanted=wanted)
    if target is None:
        return None
    try:
        resource.setrlimit(resource.RLIMIT_NOFILE, (target, hard))
    except (OSError, ValueError) as exc:
        print(
            f"warning: could not raise the file-descriptor limit from {soft} "
            f"to {target} ({exc}); a link with more objects than the limit "
            "will fail with ProcessFdQuotaExceeded",
            file=sys.stderr,
        )
        return None
    return target
