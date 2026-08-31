"""Guard the omission direction of the frontend attribution closure.

The notices artefact renders the `--production` node_modules tree, an
over-approximation of what Vite bundles. The one unsafe gap is the reverse: a
runtime dependency mis-declared under `devDependencies`, which Vite ships in the
bundle but `--production` omits from the notices. This asserts every third-party
module actually in the built bundle's module graph resolves to a package in the
`--production` closure.

The bundle graph is read from a throwaway sourcemap build in a temp directory,
never from the shipped `dist/` (whose bytes are embedded in the release binary,
so it carries no sourcemaps). Enumeration is over the sourcemap `sources`, a
structured module list, not a regex over minified output.
"""

import json
import subprocess

import pytest

from tasks.shared.paths import FRONTEND

_NODE_MODULES = "/node_modules/"
_VITE = FRONTEND / "node_modules" / ".bin" / "vite"
_LICENSE_CHECKER = (
    FRONTEND / "node_modules" / ".bin" / "license-checker-rseidelsohn"
)


def _package_of(source: str) -> str | None:
    index = source.rfind(_NODE_MODULES)
    if index == -1:
        return None
    parts = source[index + len(_NODE_MODULES) :].split("/")
    if parts[0].startswith("@"):
        return "/".join(parts[:2])
    return parts[0]


def _bundle_packages(sources: list[str]) -> set[str]:
    return {package for s in sources if (package := _package_of(s))}


def _closure_packages(license_json: str) -> set[str]:
    data = json.loads(license_json)
    return {entry["name"] for entry in data.values()}


def _assert_covered(bundle: set[str], closure: set[str]) -> None:
    missing = bundle - closure
    if missing:
        raise AssertionError(
            "bundled modules absent from the --production licence closure "
            f"(a runtime dep mis-declared under devDependencies?): "
            f"{sorted(missing)}"
        )


def _run(command: list[str]) -> subprocess.CompletedProcess:
    return subprocess.run(
        command,
        cwd=FRONTEND,
        capture_output=True,
        text=True,
        check=False,
        timeout=300,
    )


@pytest.fixture(scope="module")
def bundle_and_closure(tmp_path_factory) -> tuple[set[str], set[str]]:
    out = tmp_path_factory.mktemp("frontend-sourcemap-build")
    build = _run(
        [
            str(_VITE),
            "build",
            "--sourcemap",
            "true",
            "--outDir",
            str(out),
            "--emptyOutDir",
        ]
    )
    assert build.returncode == 0, build.stderr
    maps = sorted((out / "assets").glob("*.js.map"))
    assert maps, (
        "sourcemap build produced no .map files — cannot enumerate the bundle "
        "graph; refusing to pass vacuously"
    )
    sources: list[str] = []
    for path in maps:
        sources.extend(json.loads(path.read_text())["sources"])

    licences = _run(
        [
            str(_LICENSE_CHECKER),
            "--production",
            "--json",
            "--relativeLicensePath",
            "--excludePrivatePackages",
            "--customPath",
            "license-format.json",
        ]
    )
    assert licences.returncode == 0, licences.stderr
    payload = licences.stdout[licences.stdout.find("{") :]
    return _bundle_packages(sources), _closure_packages(payload)


def test_every_bundled_module_is_in_the_production_closure(bundle_and_closure):
    bundle, closure = bundle_and_closure
    assert bundle, "the bundle enumeration found no third-party modules"
    _assert_covered(bundle, closure)


def test_scoped_and_plain_packages_are_extracted():
    sources = [
        "../../node_modules/@tanstack/query-core/build/modern/query.js",
        "../../node_modules/react/index.js",
        "src/app.tsx",
    ]
    assert _bundle_packages(sources) == {"@tanstack/query-core", "react"}


def test_a_bundled_package_absent_from_the_closure_fails():
    with pytest.raises(AssertionError, match="rogue-dep"):
        _assert_covered({"react", "rogue-dep"}, {"react"})


def test_a_subset_bundle_passes():
    _assert_covered({"react"}, {"react", "react-dom"})
