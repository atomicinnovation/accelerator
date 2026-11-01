import ast

import pytest

from tasks.shared.paths import REPO_ROOT

_PACKAGE = "tasks.shared.build_measurement"
_PACKAGE_DIR = REPO_ROOT / "tasks" / "shared" / "build_measurement"
_SESSION = "session"
_ADAPTERS = frozenset(
    {"build_group", "build_runner", "history", "samplers", "session_lock"}
)
_COMPOSITION_ROOT = "wiring"
_PURE = sorted(
    path.stem
    for path in _PACKAGE_DIR.glob("*.py")
    if path.stem
    not in _ADAPTERS | {_SESSION, _COMPOSITION_ROOT, "__init__", "log_files"}
)


def _package_imports(module: str) -> set[str]:
    tree = ast.parse((_PACKAGE_DIR / f"{module}.py").read_text())
    return {
        node.module.removeprefix(f"{_PACKAGE}.")
        for node in ast.walk(tree)
        if isinstance(node, ast.ImportFrom)
        and node.module is not None
        and node.module.startswith(f"{_PACKAGE}.")
    }


def test_the_pure_modules_are_found():
    assert {"analysis", "measured_build", "record", "reports"} <= set(_PURE)


@pytest.mark.parametrize("module", _PURE)
def test_no_pure_module_imports_the_session_or_an_adapter(module):
    assert not _package_imports(module) & (_ADAPTERS | {_SESSION})


@pytest.mark.parametrize(
    "module",
    sorted(
        path.stem
        for path in _PACKAGE_DIR.glob("*.py")
        if path.stem != _COMPOSITION_ROOT
    ),
)
def test_only_wiring_imports_the_adapters(module):
    assert not _package_imports(module) & _ADAPTERS
