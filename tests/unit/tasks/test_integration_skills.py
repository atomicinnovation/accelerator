"""The repointed-integration-skill guards: keyword parity and the write gate.

Each guard is proven to pass on the real bodies and to fail on a committed
adversarial body — a stale keyword and a mutation-before-confirm ordering — so
neither passes vacuously.
"""

from tasks.shared import skill_keyword_parity, skill_write_gate

# A gated write body whose mutation is correctly preceded by a confirm step.
_ORDERED_BODY = """\
## Step 2: Confirm

Ask: Reply **y** to confirm.

## Step 3: Send

```
${CLAUDE_PLUGIN_ROOT}/bin/accelerator linear create <file>
```
"""

# The same body with the confirm step removed — the mutation is ungated.
_UNGATED_BODY = """\
## Step 2: Send

```
${CLAUDE_PLUGIN_ROOT}/bin/accelerator linear create <file>
```
"""

# Reversed: the mutation is invoked before the confirm ever appears.
_REVERSED_BODY = """\
## Step 2: Send

```
${CLAUDE_PLUGIN_ROOT}/bin/accelerator linear create <file>
```

Ask: Reply **y** to confirm.
"""


class TestKeywordParity:
    def test_real_bodies_pass(self):
        assert skill_keyword_parity.violations() == []

    def test_a_stale_keyword_is_flagged(self):
        declared = {"created", "not-found"}
        text = "On `frobnicate`, do the thing (the outcome keyword)."
        unknown = skill_keyword_parity.referenced_keywords(text) - declared
        assert unknown == {"frobnicate"}

    def test_a_real_keyword_is_not_flagged(self):
        declared = {"created", "not-found"}
        text = "When `outcome` is `not-found`, tell the user."
        assert skill_keyword_parity.referenced_keywords(text) <= declared


class TestWriteGate:
    def test_real_bodies_pass(self):
        assert skill_write_gate.violations() == []

    def test_an_ordered_body_passes(self):
        assert (
            skill_write_gate.body_violations(_ORDERED_BODY, "linear", "fix")
            == []
        )

    def test_a_reversed_body_is_flagged(self):
        found = skill_write_gate.body_violations(
            _REVERSED_BODY, "linear", "fixture"
        )
        assert found and "no confirm step precedes" in found[0]

    def test_an_ungated_mutation_is_flagged(self):
        found = skill_write_gate.body_violations(
            _UNGATED_BODY, "linear", "fixture"
        )
        assert found and "no confirm step precedes" in found[0]

    def test_a_read_body_is_out_of_scope(self):
        read_body = (
            "```\n${CLAUDE_PLUGIN_ROOT}/bin/accelerator linear show X\n```"
        )
        assert (
            skill_write_gate.body_violations(read_body, "linear", "fix") is None
        )
