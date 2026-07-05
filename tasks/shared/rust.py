import os

LAUNCHER_CRATE = "accelerator"  # cli/launcher/Cargo.toml [package] name
# PUP_NIGHTLY + PUP_VERSION are a matched pair (cargo-pup's rustc-driver only
# loads under the nightly it was built against); bump them together.
PUP_NIGHTLY = "nightly-2026-01-22"  # cargo-pup v0.1.8 rust-toolchain.toml
PUP_VERSION = "0.1.8"

_FALSEY = {"off", "false", "0", "no"}
_PUP_MODES = {"deny", "warn"}


def coverage_enabled() -> bool:
    """Whether cli tests run instrumented. Read at CALL time, never at import.

    True -> `cargo llvm-cov nextest` (coverage reported); False -> plain
    `cargo nextest run` (faster inner loop). Env-sourced so a developer can
    drop coverage without a source edit; CI leaves it on. Must be called inside
    the task body — a module-level constant would freeze the value at import.
    Any of off/false/0/no (case-insensitive) disables it, so a plausible falsey
    value does not silently leave the slow path on.
    """
    raw = os.environ.get("ACCELERATOR_COVERAGE", "on").strip().lower()
    return raw not in _FALSEY


def pup_mode() -> str:
    """cargo-pup blocking mode. Read at CALL time, never at import.

    "deny" -> fail on findings (blocking); "warn" -> advisory (log only).
    Default "deny" is fail-closed. The value is normalised (strip + lower-case)
    so an incident-time typo like "Warn"/" warn " still activates the escape
    hatch; an unrecognised value is treated as "deny" (fail-closed) but printed
    as a WARNING so the typo is visible rather than silently blocking. NOTE:
    warn covers a cargo-pup *findings* failure, not a toolchain-*unavailable*
    failure (which fails in deps:install:pup before any check runs).
    """
    raw = os.environ.get("ACCELERATOR_PUP_MODE", "deny").strip().lower()
    if raw not in _PUP_MODES:
        print(
            f"WARNING: unrecognised ACCELERATOR_PUP_MODE={raw!r}; using 'deny'"
        )
        return "deny"
    return raw
