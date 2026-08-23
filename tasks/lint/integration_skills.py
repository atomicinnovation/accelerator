"""Guard the repointed integration skills' outcome contract.

Two invariants the jira/linear cutover depends on, checked together because both
parse the same ``skills/integrations/*/SKILL.md`` bodies:

- **keyword parity** — every outcome keyword a body branches on exists in the
  sub-binary's declared keyword set, no body cites a bash exit integer or a
  dropped ``--print-payload``/``--describe`` preview (Decision 11);
- **write gate** — every write skill's confirm step precedes its
  ``accelerator <provider> <mutation>`` invocation (Decision 2).
"""

from invoke import Context, Exit, task

from tasks.shared import skill_keyword_parity, skill_write_gate


@task
def check(context: Context) -> None:
    """Fail if a repointed integration skill breaks its outcome contract."""
    offenders = (
        skill_keyword_parity.violations() + skill_write_gate.violations()
    )
    if offenders:
        raise Exit(
            "check-integration-skills found problem(s):\n  "
            + "\n  ".join(offenders),
            code=1,
        )
