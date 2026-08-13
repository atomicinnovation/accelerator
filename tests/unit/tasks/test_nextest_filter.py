"""Guard the contract-suite nextest filter.

Its own file rather than `test_rust.py`, following the precedent
`test_registration_docs.py` states in its own docstring: guards over
non-Python artefacts live in dedicated files beside `test_mise.py` and
`test_workflows.py`. `test_rust.py`'s scope is the `tasks/shared/rust.py`
helpers and the task leaves, so a guard over a Rust-tooling config file is
invisible there to both the config's maintainer and the task's.

Without this, the next author to name a test binary `contract.rs` in some
other crate gets it silently dropped from `mise run` with no signal, and a
`--profile`/`--ignore-default-filter` slipped into `test:unit:cli`'s command
would bypass the filter with nothing catching it — from 0171 onward that
means live API calls in the default test run.
"""

import tomllib

from tasks.shared.sources import repo_root

REPO_ROOT = repo_root()
NEXTEST_TOML = REPO_ROOT / "cli/.config/nextest.toml"
CLI_PY = REPO_ROOT / "tasks/test/cli.py"


def test_default_profile_excludes_the_contract_binary_by_exact_match():
    config = tomllib.loads(NEXTEST_TOML.read_text())
    default_filter = config["profile"]["default"]["default-filter"]

    assert default_filter == "not binary(=contract)", (
        "the default profile's filter has drifted from the exact-match form "
        "— bare binary(contract) is a substring predicate that would "
        "silently pull a future contract_helpers/contract_smoke binary into "
        "the contract profile too"
    )


def test_contract_profile_selects_exactly_the_contract_binary():
    config = tomllib.loads(NEXTEST_TOML.read_text())
    contract_filter = config["profile"]["contract"]["default-filter"]

    assert contract_filter == "binary(=contract)"


def test_the_cli_test_command_does_not_bypass_the_default_filter():
    source = CLI_PY.read_text()

    assert "--profile" not in source, (
        "tasks/test/cli.py passes --profile, which would select a nextest "
        "profile other than default and bypass the contract filter"
    )
    assert "--ignore-default-filter" not in source, (
        "tasks/test/cli.py passes --ignore-default-filter, which would run "
        "the contract binary inside test:unit:cli"
    )
