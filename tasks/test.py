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

    print("Running ticket script tests...")
    context.run("skills/tickets/scripts/test-ticket-scripts.sh")
    print("\n")

    print("Running lens structure lint...")
    context.run("scripts/test-lens-structure.sh")
    print("\n")
