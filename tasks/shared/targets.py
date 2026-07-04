TARGETS = (
    ("aarch64-apple-darwin", "darwin-arm64"),
    ("x86_64-apple-darwin", "darwin-x64"),
    ("aarch64-unknown-linux-musl", "linux-arm64"),
    ("x86_64-unknown-linux-musl", "linux-x64"),
)

# The four platform aliases, single-sourced from TARGETS.
ALIASES = tuple(alias for _triple, alias in TARGETS)

# Canonical `uname`-input → platform-alias mapping. `targets.py`'s TARGETS maps
# Rust triples → aliases; this maps the (uname -s, uname -m) spellings the
# launcher (HOST_PLATFORM cfg) and the bootstrap (`case` arms) must both
# normalise. It is the oracle the cross-language coherence test asserts those
# two tables against, so a bootstrap arm that fails to normalise amd64/aarch64
# fails the test rather than 404-ing on a user's host. Keys are (os, machine)
# with `os` lowercased from `uname -s` and `machine` the raw `uname -m`.
UNAME_TO_ALIAS = {
    ("darwin", "arm64"): "darwin-arm64",
    ("darwin", "aarch64"): "darwin-arm64",
    ("darwin", "x86_64"): "darwin-x64",
    ("darwin", "amd64"): "darwin-x64",
    ("linux", "arm64"): "linux-arm64",
    ("linux", "aarch64"): "linux-arm64",
    ("linux", "x86_64"): "linux-x64",
    ("linux", "amd64"): "linux-x64",
}
