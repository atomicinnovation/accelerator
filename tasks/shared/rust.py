import os

LAUNCHER_CRATE = "accelerator"  # cli/launcher/Cargo.toml [package] name

# The one nightly toolchain this repository provisions, for the build steps
# that need a capability stable does not expose: the `rustc_private` compiler
# internals a compiler plugin links against, and rustdoc's JSON output. It is
# rustup-managed (mise cannot pin two rust toolchains) and always invoked as
# `cargo +RUST_NIGHTLY`. Nothing else — no product build, no other check —
# leaves the mise-pinned stable.
#
# Its date is dictated by the strongest coupling below, not chosen freely.
RUST_NIGHTLY = "nightly-2026-01-22"

# cargo-pup — the architecture check (pup:check, test:integration:pup).
# A matched pair with RUST_NIGHTLY: the tool carries a `rustc_private` driver,
# so its binary only loads under the nightly it was *built* against. Bump the
# two together, taking the date from the cargo-pup release's own
# rust-toolchain.toml.
PUP_VERSION = "0.1.8"

# cargo-public-api — the surface pin (public-api:check).
# Coupled to RUST_NIGHTLY only through the rustdoc-JSON format: the tool has no
# driver, so it builds on stable and merely shells out to the nightly's
# `rustdoc`. After a RUST_NIGHTLY bump, re-verify this pin supports the new
# nightly's JSON format before accepting a snapshot diff as toolchain-induced.
PUBLIC_API_VERSION = "0.52.0"

# cargo-about — the third-party notices generator (notices:check,
# notices:update). Built from source on stable rather than pinned as a mise
# [tool]: its 0.9.x release binaries omit x86_64-apple-darwin, so a ubi pin
# would fail `mise install` on the Intel-mac CI leg. The source build resolves
# on every host, matching the cargo-public-api provisioning pattern. This is a
# fourth accepted unverified surface beside cargo-pup/cargo-public-api.
ABOUT_VERSION = "0.9.2"

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
    failure (which fails in deps:install:nightly before any check runs).
    """
    raw = os.environ.get("ACCELERATOR_PUP_MODE", "deny").strip().lower()
    if raw not in _PUP_MODES:
        print(
            f"WARNING: unrecognised ACCELERATOR_PUP_MODE={raw!r}; using 'deny'"
        )
        return "deny"
    return raw
