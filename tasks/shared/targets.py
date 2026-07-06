import platform as _platform

TARGETS = (
    ("aarch64-apple-darwin", "darwin-arm64"),
    ("x86_64-apple-darwin", "darwin-x64"),
    ("aarch64-unknown-linux-musl", "linux-arm64"),
    ("x86_64-unknown-linux-musl", "linux-x64"),
)

ALIASES = tuple(alias for _triple, alias in TARGETS)

# (uname -s lowercased, uname -m) -> platform alias. The launcher and bootstrap
# normalise the same spellings; the coherence test asserts all three agree.
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


def host_platform() -> str:
    """Return the platform alias of the host running this process.

    Used to pick the runner-arch verify shim for the release re-verify step
    (macos-latest is darwin-arm64). Raises on an unsupported host rather than
    guessing.
    """
    key = (_platform.system().lower(), _platform.machine().lower())
    try:
        return UNAME_TO_ALIAS[key]
    except KeyError:
        raise RuntimeError(f"unsupported host platform: {key}") from None
