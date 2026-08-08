"""Tests for the generalised dispatch-coherence guard.

Every case injects fixture tokens through the guard's parameters; none touches
`DISPATCHED_SUBBINARIES`, so the real signing, manifest and upload paths stay
untouched by the verification. Each raising case asserts a discriminating
substring where one exists; where several cases share a message, the docstring
names the mutation the case kills.
"""

import inspect
import re
from pathlib import Path

import pytest

from tasks.shared import dispatch_coherence as guard
from tasks.shared.dispatch_coherence import (
    BUILTIN_SUBCOMMANDS,
    RESERVED_TOKENS,
    validate_dispatch_coherence,
    violations,
)
from tasks.shared.paths import DISPATCHED_SUBBINARIES, SKILL_EXEMPT_SUBBINARIES
from tasks.shared.skill_parsing import LAUNCHER, PLUGIN_PREFIX
from tasks.shared.sources import repo_root

REPO_ROOT = repo_root()

_TOK = "frobnicate"
_OTHER = "widgetise"

_GUARD_SOURCE = REPO_ROOT / "tasks/shared/dispatch_coherence.py"
_CLI_RS = REPO_ROOT / "cli/launcher/src/launch/inbound/cli.rs"
_MAIN_RS = REPO_ROOT / "cli/launcher/src/main.rs"
_CORE_RS = REPO_ROOT / "cli/launcher/src/launch/core.rs"


def _skill(
    root: Path,
    rel: str,
    *,
    rules: tuple[str, ...] = (),
    commands: tuple[str, ...] = (),
    prose: str = "",
    bare_bash: bool = False,
) -> None:
    path = root / "skills" / rel / "SKILL.md"
    path.parent.mkdir(parents=True, exist_ok=True)
    allowed = [f"Bash({rule})" for rule in rules]
    if bare_bash:
        allowed.insert(0, "Bash")
    tools = "".join(f"\n  - {entry}" for entry in allowed)
    body = "\n".join([*(f"!`{command}`" for command in commands), prose])
    path.write_text(f"---\nname: {rel}\nallowed-tools:{tools}\n---\n{body}\n")


def _bound_skill(root: Path, token: str = _TOK, rel: str = "consumer") -> None:
    _skill(
        root,
        rel,
        rules=(f"{LAUNCHER} {token} *",),
        commands=(f"{LAUNCHER} {token} start",),
    )


class TestBinding:
    def test_a_scoped_rule_and_a_real_invocation_bind(
        self, tmp_path: Path
    ) -> None:
        _bound_skill(tmp_path)
        assert violations(tmp_path, tokens=(_TOK,), exempt=()) == []

    def test_a_rule_scoped_tighter_than_the_token_binds(
        self, tmp_path: Path
    ) -> None:
        _skill(
            tmp_path,
            "consumer",
            rules=(f"{LAUNCHER} {_TOK} start",),
            commands=(f"{LAUNCHER} {_TOK} start",),
        )
        assert violations(tmp_path, tokens=(_TOK,), exempt=()) == []

    def test_a_flag_glob_rule_covering_the_invocation_binds(
        self, tmp_path: Path
    ) -> None:
        _skill(
            tmp_path,
            "consumer",
            rules=(f"{LAUNCHER} {_TOK} --owner-pid *",),
            commands=(f"{LAUNCHER} {_TOK} --owner-pid 42",),
        )
        assert violations(tmp_path, tokens=(_TOK,), exempt=()) == []

    def test_a_rule_not_covering_the_invocation_does_not_bind(
        self, tmp_path: Path
    ) -> None:
        """Kills a dropped `covered_by(command, rule)` conjunct."""
        _skill(
            tmp_path,
            "consumer",
            rules=(f"{LAUNCHER} {_TOK} start",),
            commands=(f"{LAUNCHER} {_TOK} status",),
        )
        assert violations(tmp_path, tokens=(_TOK,), exempt=()) != []

    def test_the_two_conditions_cannot_be_split_across_rules(
        self, tmp_path: Path
    ) -> None:
        """Kills evaluating segment-equality and coverage on separate rules."""
        _skill(
            tmp_path,
            "consumer",
            rules=(f"{LAUNCHER} {_TOK} start", f"{LAUNCHER} {_TOK[:4]}*"),
            commands=(f"{LAUNCHER} {_TOK} status",),
        )
        assert violations(tmp_path, tokens=(_TOK,), exempt=()) != []

    def test_two_consumers_bind_when_one_is_correctly_scoped(
        self, tmp_path: Path
    ) -> None:
        _bound_skill(tmp_path, rel="good")
        _skill(
            tmp_path,
            "loose",
            rules=(f"{LAUNCHER} *",),
            commands=(f"{LAUNCHER} {_TOK} status",),
        )
        assert violations(tmp_path, tokens=(_TOK,), exempt=()) == []


class TestMissingBinding:
    def test_a_token_with_no_consuming_skill_fails(
        self, tmp_path: Path
    ) -> None:
        (tmp_path / "skills").mkdir()
        problems = violations(tmp_path, tokens=(_TOK,), exempt=())
        assert any("no skill invokes" in p for p in problems)

    def test_only_the_unbound_token_of_two_is_named(
        self, tmp_path: Path
    ) -> None:
        _bound_skill(tmp_path, token=_OTHER)
        problems = violations(tmp_path, tokens=(_OTHER, _TOK), exempt=())
        assert len(problems) == 1
        assert _TOK in problems[0]
        assert _OTHER not in problems[0]

    def test_prose_and_backticks_do_not_bind(self, tmp_path: Path) -> None:
        _skill(
            tmp_path,
            "consumer",
            rules=(f"{LAUNCHER} {_TOK} *",),
            prose=f"Run `{LAUNCHER} {_TOK} start` to begin, or {_TOK} start.",
        )
        problems = violations(tmp_path, tokens=(_TOK,), exempt=())
        assert any("no skill invokes" in p for p in problems)

    def test_a_different_bound_token_does_not_bind_the_target(
        self, tmp_path: Path
    ) -> None:
        _bound_skill(tmp_path, token=_OTHER)
        problems = violations(tmp_path, tokens=(_TOK, _OTHER), exempt=())
        assert any("no skill invokes" in p for p in problems)
        assert not any("neither dispatched" in p for p in problems)

    def test_a_chained_invocation_does_not_bind(self, tmp_path: Path) -> None:
        """Kills a dropped `has_metacharacter` skip.

        The permissive scan still records the token, so the message names the
        missing rule rather than a missing invocation.
        """
        _skill(
            tmp_path,
            "consumer",
            rules=(f"{LAUNCHER} {_TOK} *",),
            commands=(f"{LAUNCHER} {_TOK} status && rm -rf x",),
        )
        problems = violations(tmp_path, tokens=(_TOK,), exempt=())
        assert any("declares no Bash(...) rule" in p for p in problems)


class TestOverBroadSkills:
    @pytest.mark.parametrize(
        "rule",
        [
            f"{LAUNCHER} *",
            f"{PLUGIN_PREFIX}bin/*",
            f"{PLUGIN_PREFIX}*",
        ],
    )
    def test_an_ancestor_glob_alone_does_not_bind(
        self, tmp_path: Path, rule: str
    ) -> None:
        _skill(
            tmp_path,
            "consumer",
            rules=(rule,),
            commands=(f"{LAUNCHER} {_TOK} start",),
        )
        problems = violations(tmp_path, tokens=(_TOK,), exempt=())
        assert any("skills/consumer/SKILL.md" in p for p in problems)

    def test_an_ancestor_glob_disqualifies_the_whole_skill(
        self, tmp_path: Path
    ) -> None:
        """Kills a per-rule veto: the sentinel check is skill-level."""
        _skill(
            tmp_path,
            "consumer",
            rules=(f"{LAUNCHER} {_TOK} *", f"{LAUNCHER} *"),
            commands=(f"{LAUNCHER} {_TOK} start",),
        )
        assert violations(tmp_path, tokens=(_TOK,), exempt=()) != []

    def test_a_charset_glob_disqualifies_the_whole_skill(
        self, tmp_path: Path
    ) -> None:
        """Kills the charset half of the veto.

        `[a-y]*` pre-authorises every token not starting with `z`, but the
        `zz-`-prefixed bare-launcher sentinel does not match it.
        """
        _skill(
            tmp_path,
            "consumer",
            rules=(f"{LAUNCHER} {_TOK} *", f"{LAUNCHER} [a-y]*"),
            commands=(f"{LAUNCHER} {_TOK} start",),
        )
        assert violations(tmp_path, tokens=(_TOK,), exempt=()) != []

    def test_a_wildcarded_token_segment_does_not_bind(
        self, tmp_path: Path
    ) -> None:
        _skill(
            tmp_path,
            "consumer",
            rules=(f"{LAUNCHER} {_TOK[:4]}*",),
            commands=(f"{LAUNCHER} {_TOK} start",),
        )
        assert violations(tmp_path, tokens=(_TOK,), exempt=()) != []

    def test_a_rule_for_a_different_subcommand_does_not_bind(
        self, tmp_path: Path
    ) -> None:
        """Kills a dropped segment-equality conjunct."""
        _skill(
            tmp_path,
            "consumer",
            rules=(f"{LAUNCHER} config *",),
            commands=(f"{LAUNCHER} {_TOK} start",),
        )
        assert violations(tmp_path, tokens=(_TOK,), exempt=()) != []

    def test_a_verify_shim_rule_does_not_bind(self, tmp_path: Path) -> None:
        _skill(
            tmp_path,
            "consumer",
            rules=(f"{PLUGIN_PREFIX}bin/accelerator-verify *",),
            commands=(f"{LAUNCHER} {_TOK} start",),
        )
        assert violations(tmp_path, tokens=(_TOK,), exempt=()) != []

    def test_bare_bash_alone_does_not_bind(self, tmp_path: Path) -> None:
        _skill(
            tmp_path,
            "consumer",
            bare_bash=True,
            commands=(f"{LAUNCHER} {_TOK} start",),
        )
        problems = violations(tmp_path, tokens=(_TOK,), exempt=())
        assert any("declares no Bash(...) rule" in p for p in problems)

    def test_bare_bash_disqualifies_a_scoped_rule_beside_it(
        self, tmp_path: Path
    ) -> None:
        _skill(
            tmp_path,
            "consumer",
            rules=(f"{LAUNCHER} {_TOK} *",),
            bare_bash=True,
            commands=(f"{LAUNCHER} {_TOK} start",),
        )
        assert violations(tmp_path, tokens=(_TOK,), exempt=()) != []


class TestInvocationToRegistration:
    def test_an_unregistered_token_fails(self, tmp_path: Path) -> None:
        _bound_skill(tmp_path)
        _skill(
            tmp_path,
            "stray",
            rules=(f"{LAUNCHER} zz-unregistered-zz *",),
            commands=(f"{LAUNCHER} zz-unregistered-zz go",),
        )
        problems = violations(tmp_path, tokens=(_TOK,), exempt=())
        assert any(
            "neither dispatched" in p and "skills/stray/SKILL.md" in p
            for p in problems
        )

    def test_a_mid_chain_invocation_is_still_seen(self, tmp_path: Path) -> None:
        """The invocation half is deliberately permissive — it fails closed."""
        _bound_skill(tmp_path)
        _skill(
            tmp_path,
            "stray",
            rules=(f"{LAUNCHER} zz-unregistered-zz *",),
            commands=(f"cd . && {LAUNCHER} zz-unregistered-zz go",),
        )
        problems = violations(tmp_path, tokens=(_TOK,), exempt=())
        assert any("neither dispatched" in p for p in problems)

    @pytest.mark.parametrize("builtin", sorted(BUILTIN_SUBCOMMANDS))
    def test_a_builtin_needs_no_registration(
        self, tmp_path: Path, builtin: str
    ) -> None:
        _bound_skill(tmp_path)
        _skill(
            tmp_path,
            "builtin-user",
            rules=(f"{LAUNCHER} {builtin} *",),
            commands=(f"{LAUNCHER} {builtin} get x",),
        )
        assert violations(tmp_path, tokens=(_TOK,), exempt=()) == []

    def test_a_flag_first_argument_is_not_a_token(self, tmp_path: Path) -> None:
        _bound_skill(tmp_path)
        _skill(
            tmp_path,
            "flagger",
            rules=(f"{LAUNCHER} --version",),
            commands=(f"{LAUNCHER} --version",),
        )
        assert violations(tmp_path, tokens=(_TOK,), exempt=()) == []

    def test_a_sibling_binary_contributes_no_token(
        self, tmp_path: Path
    ) -> None:
        _bound_skill(tmp_path)
        _skill(
            tmp_path,
            "verifier",
            rules=(f"{PLUGIN_PREFIX}bin/accelerator-verify-* *",),
            commands=(
                f"{PLUGIN_PREFIX}bin/accelerator-verify-darwin-arm64 x.json",
            ),
        )
        assert violations(tmp_path, tokens=(_TOK,), exempt=()) == []


class TestExemptions:
    def test_an_exempt_token_with_no_consumer_passes(
        self, tmp_path: Path
    ) -> None:
        # Two tokens: with one token and one exemption the all-exempt rule
        # fires first, so a single-token exemption can never pass.
        _bound_skill(tmp_path, token=_OTHER)
        assert violations(tmp_path, tokens=(_TOK, _OTHER), exempt=(_TOK,)) == []

    def test_the_same_token_without_the_exemption_fails(
        self, tmp_path: Path
    ) -> None:
        _bound_skill(tmp_path, token=_OTHER)
        problems = violations(tmp_path, tokens=(_TOK, _OTHER), exempt=())
        assert any("no skill invokes" in p for p in problems)

    def test_an_invoked_exempt_token_fails(self, tmp_path: Path) -> None:
        _bound_skill(tmp_path, token=_OTHER)
        _bound_skill(tmp_path, token=_TOK, rel="consumer-two")
        problems = violations(tmp_path, tokens=(_TOK, _OTHER), exempt=(_TOK,))
        assert any("exempt but" in p for p in problems)

    def test_an_exempt_token_invoked_mid_chain_fails(
        self, tmp_path: Path
    ) -> None:
        _bound_skill(tmp_path, token=_OTHER)
        _skill(
            tmp_path,
            "chained",
            rules=(f"{LAUNCHER} {_TOK} *",),
            commands=(f"cd . && {LAUNCHER} {_TOK} start",),
        )
        problems = violations(tmp_path, tokens=(_TOK, _OTHER), exempt=(_TOK,))
        assert any("exempt but" in p for p in problems)

    def test_an_exemption_naming_an_undispatched_token_fails(
        self, tmp_path: Path
    ) -> None:
        _bound_skill(tmp_path)
        problems = violations(tmp_path, tokens=(_TOK,), exempt=(_OTHER,))
        assert any("exempt but not dispatched" in p for p in problems)

    def test_an_all_exempt_collection_fails(self, tmp_path: Path) -> None:
        problems = violations(tmp_path, tokens=(_TOK,), exempt=(_TOK,))
        assert any(
            "every dispatched sub-binary is exempt" in p for p in problems
        )


class TestRegistryBounds:
    def test_an_empty_collection_fails(self, tmp_path: Path) -> None:
        problems = violations(tmp_path, tokens=(), exempt=())
        assert any(
            "lost rather than deliberately emptied" in p for p in problems
        )

    @pytest.mark.parametrize("token", sorted(RESERVED_TOKENS))
    def test_a_reserved_token_fails(self, tmp_path: Path, token: str) -> None:
        problems = violations(tmp_path, tokens=(token,), exempt=())
        assert any("reserved" in p for p in problems)

    @pytest.mark.parametrize("token", sorted(BUILTIN_SUBCOMMANDS))
    def test_a_builtin_shadowing_token_fails(
        self, tmp_path: Path, token: str
    ) -> None:
        problems = violations(tmp_path, tokens=(token,), exempt=())
        assert any("shadows a launcher built-in" in p for p in problems)

    @pytest.mark.parametrize("token", ["frob_thing", "2fast", "Vcs", "-lead"])
    def test_an_invalid_charset_fails(self, tmp_path: Path, token: str) -> None:
        problems = violations(tmp_path, tokens=(token,), exempt=())
        assert any("not a valid token" in p for p in problems)

    @pytest.mark.parametrize("token", ["vcs", "work-item", "a", "s3"])
    def test_a_valid_charset_reaches_the_skills_scan(
        self, tmp_path: Path, token: str
    ) -> None:
        _bound_skill(tmp_path, token=token)
        assert violations(tmp_path, tokens=(token,), exempt=()) == []


# Tracked, individually-justified tokens with a real, planned SKILL.md
# consumer that is not wired yet — distinct from SKILL_EXEMPT_SUBBINARIES,
# whose own docstring reserves it for a token no SKILL.md will ever invoke.
# Each entry here must name the work landing the real binding, so this stays a
# visible, deliberate carve-out rather than a place a careless exemption can
# hide: adding a token to SKILL_EXEMPT_SUBBINARIES does NOT exempt it here.
#
# migrate: work-item:0172 Phase 1 registers the sub-binary ahead of Phase 7's
# skill rebinding (skills/config/migrate/SKILL.md still shells out to
# run-migrations.sh, which Phase 7 replaces once migrations are registered
# and interactive support lands). Remove once that rebinding lands.
_KNOWN_PENDING_SKILL_BINDINGS = ("migrate",)


def test_the_real_skills_tree_passes() -> None:
    # Every dispatched token not in the tracked-pending set above must have a
    # real skill binding, regardless of what SKILL_EXEMPT_SUBBINARIES says —
    # so no future addition to that constant can make the one production
    # binding vacuous.
    assert violations(REPO_ROOT, exempt=_KNOWN_PENDING_SKILL_BINDINGS) == []


def test_validate_raises_with_every_problem(tmp_path: Path) -> None:
    (tmp_path / "skills").mkdir()
    with pytest.raises(guard.DispatchCoherenceError, match="no skill invokes"):
        validate_dispatch_coherence(tmp_path, tokens=(_TOK,), exempt=())


def test_validate_is_silent_when_coherent(tmp_path: Path) -> None:
    _bound_skill(tmp_path)
    validate_dispatch_coherence(tmp_path, tokens=(_TOK,), exempt=())


class TestSourceScans:
    def test_no_hardcoded_visualiser_skill_path_under_tasks(self) -> None:
        offenders = [
            path.relative_to(REPO_ROOT).as_posix()
            for path in sorted((REPO_ROOT / "tasks").rglob("*.py"))
            if "_VISUALISE_SKILL_RELATIVE" in path.read_text()
        ]
        assert offenders == []

    def test_the_guard_names_no_token(self) -> None:
        assert "visualiser" not in _GUARD_SOURCE.read_text()
        # Positive control: the same predicate over the registry module, which
        # legitimately names the token, must report it.
        assert "visualiser" in (REPO_ROOT / "tasks/shared/paths.py").read_text()

    def test_the_guard_imports_the_shared_parsing(self) -> None:
        source = _GUARD_SOURCE.read_text()
        names = (
            "LAUNCHER",
            "preprocessor_commands",
            "frontmatter_bash_rules",
            "has_bare_bash",
            "has_metacharacter",
            "is_plugin_invocation",
            "covered_by",
            "launcher_token",
            "BARE_LAUNCHER",
        )
        block = re.search(
            r"from tasks\.shared\.skill_parsing import \(([^)]*)\)", source
        )
        assert block, "the guard must import from tasks.shared.skill_parsing"
        imported = {n.strip().rstrip(",") for n in block.group(1).split()}
        assert set(names) <= imported, sorted(set(names) - imported)
        body = source[block.end() :]
        for name in names:
            assert name in body, f"{name} is imported but never used"
            assert not re.search(rf"^\s*{name} =", body, re.MULTILINE)
            assert not re.search(rf"^\s*def {name}\b", body, re.MULTILINE)

    def test_the_guard_imports_no_private_parsing_name(self) -> None:
        source = _GUARD_SOURCE.read_text()
        block = re.search(
            r"from tasks\.shared\.skill_parsing import \(([^)]*)\)", source
        )
        assert block
        assert not any(
            n.strip().startswith("_") for n in block.group(1).split()
        )

    def test_the_guard_compiles_no_matcher_of_its_own(self) -> None:
        source = _GUARD_SOURCE.read_text()
        assert "fnmatch" not in source
        patterns = re.findall(r"re\.compile\((.*)\)", source)
        assert patterns, "the token charset regex should still be here"
        for pattern in patterns:
            assert "Bash" not in pattern
            assert "!`" not in pattern

    def test_the_guard_imports_no_lint_module_or_invoke(self) -> None:
        source = _GUARD_SOURCE.read_text()
        imports = set(
            re.findall(r"^(?:from|import) (\S+)", source, re.MULTILINE)
        )
        assert not any(
            i == "invoke" or i.startswith(("invoke.", "tasks.lint"))
            for i in imports
        )


@pytest.mark.parametrize(
    "entry_point", [violations, validate_dispatch_coherence]
)
def test_the_defaults_are_the_real_constants(entry_point) -> None:
    parameters = inspect.signature(entry_point).parameters
    assert parameters["tokens"].default is DISPATCHED_SUBBINARIES
    assert parameters["exempt"].default is SKILL_EXEMPT_SUBBINARIES


def test_both_signatures_literally_name_the_exemption_constant() -> None:
    # SKILL_EXEMPT_SUBBINARIES is `()` and CPython interns the empty tuple, so
    # the identity assertion above is degenerate while the set is empty.
    source = _GUARD_SOURCE.read_text()
    assert source.count("exempt: Iterable[str] = SKILL_EXEMPT_SUBBINARIES") == 2
    assert source.count("tokens: Iterable[str] = DISPATCHED_SUBBINARIES") == 2


def _command_variants(text: str) -> set[str]:
    body = re.search(
        r"pub enum Command \{(.*?)^\}", text, re.DOTALL | re.MULTILINE
    )
    assert body, "the Command enum was not found"
    return set(re.findall(r"^    ([A-Z]\w*)", body.group(1), re.MULTILINE))


class TestCrossLanguagePins:
    def test_builtin_subcommands_match_the_clap_command_enum(self) -> None:
        text = _CLI_RS.read_text()
        variants = _command_variants(text)
        assert variants == {"Version", "Config", "External"}
        dispatchable = {v.lower() for v in variants if v != "External"}
        assert dispatchable | {"help"} == set(BUILTIN_SUBCOMMANDS)

    def test_the_variant_extractor_sees_an_added_command(self) -> None:
        mutated = _CLI_RS.read_text().replace(
            "pub enum Command {", "pub enum Command {\n    Vcs,", 1
        )
        assert "Vcs" in _command_variants(mutated)

    def test_no_command_variant_carries_a_clap_alias(self) -> None:
        text = _CLI_RS.read_text()
        body = re.search(
            r"pub enum Command \{(.*?)^\}", text, re.DOTALL | re.MULTILINE
        )
        assert body
        for attribute in ("name =", "alias =", "visible_alias ="):
            assert attribute not in body.group(1)

    def test_is_root_help_agrees_as_a_secondary_check(self) -> None:
        match = re.search(
            r'Some\((\s*"[a-z]+"(?:\s*\|\s*"[a-z]+")*)\)', _MAIN_RS.read_text()
        )
        assert match
        assert set(re.findall(r'"([a-z]+)"', match.group(1))) == set(
            BUILTIN_SUBCOMMANDS
        )

    @pytest.mark.parametrize(
        ("token", "accepted"),
        [
            ("vcs", True),
            ("work-item", True),
            ("frob_thing", False),
            ("2fast", False),
            ("Vcs", False),
        ],
    )
    def test_the_token_charset_matches_derive_override_var(
        self, token: str, accepted: bool
    ) -> None:
        constraint = (
            "must start with a letter and contain only letters, \\\n"
            "                     digits, and hyphens"
        )
        assert constraint in _CORE_RS.read_text()
        assert bool(guard._TOKEN.match(token)) is accepted

    def test_reserved_tokens_are_the_staged_but_undispatched_binaries(
        self,
    ) -> None:
        from tasks.build import _CLI_RELEASE_BINARIES

        staged = {
            name.removeprefix("accelerator-")
            for name in _CLI_RELEASE_BINARIES
            if name != "accelerator"
        }
        # The subtraction is load-bearing: the checklist tells every author to
        # add `accelerator-<token>` to _CLI_RELEASE_BINARIES, so without it the
        # first sibling story would reserve its own token.
        expected = (staged - set(DISPATCHED_SUBBINARIES)) | {"launcher"}
        assert set(RESERVED_TOKENS) == expected
