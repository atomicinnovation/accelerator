from invoke import Context, task

@task
def integration(context: Context):
    """Run integration tests."""
    print("Running configuration script tests...")
    context.run("scripts/test-config.sh")
    print("\n")

    print("Running ADR script tests...")
    context.run("skills/decisions/scripts/test-adr-scripts.sh")
    print("\n")

    print("Running work item script tests...")
    context.run("skills/work/scripts/test-work-item-scripts.sh")
    print("\n")

    print("Running lens structure lint...")
    context.run("scripts/test-lens-structure.sh")
    print("\n")

    print("Running lens boundary eval checks...")
    context.run("scripts/test-boundary-evals.sh")
    print("\n")

    print("Running eval structure self-test...")
    context.run("scripts/test-evals-structure-self.sh")
    print("\n")

    print("Running eval structure validation...")
    context.run("scripts/test-evals-structure.sh")
    print("\n")

    print("Running hierarchy format drift check...")
    context.run("scripts/test-hierarchy-format.sh")
    print("\n")

    print("Running format checks...")
    context.run("scripts/test-format.sh")
    print("\n")

    print("Running migration framework tests...")
    context.run("skills/config/migrate/scripts/test-migrate.sh")
    print("\n")
