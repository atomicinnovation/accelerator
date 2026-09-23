from invoke import Collection

from . import (
    build,
    changelog,
    deny,
    deps,
    dev,
    docs,
    git,
    github,
    lint,
    marketplace,
    measure,
    notices,
    public_api,
    pup,
    release,
    signing,
    test,
    types,
    version,
)
from . import format as format_  # `format` shadows a builtin; alias the import
from .vendor import commands as vendor_commands

ns = Collection()

ns_prerelease = Collection("prerelease")
ns_prerelease.add_task(release.prerelease, default=True)
ns_prerelease.add_task(release.prerelease_prepare, name="prepare")
ns_prerelease.add_task(release.prerelease_sign, name="sign")
ns_prerelease.add_task(release.prerelease_finalise, name="finalise")
ns.add_collection(ns_prerelease)

ns_release = Collection("release")
ns_release.add_task(release.release, default=True)
ns_release.add_task(release.release_prepare, name="prepare")
ns_release.add_task(release.release_sign, name="sign")
ns_release.add_task(release.release_finalise, name="finalise")
ns.add_collection(ns_release)

ns.add_collection(Collection.from_module(build))

ns_keys = Collection("keys")
ns_keys.add_task(signing.generate, name="generate")
ns.add_collection(ns_keys)

# Manual dev collection so a bare `invoke dev` maps to `up` (the unified
# supervised stack), while the manual two-terminal tasks remain available.
ns_dev = Collection("dev")
ns_dev.add_task(dev.up, default=True)
ns_dev.add_task(dev.stop)
ns_dev.add_task(dev.restart)
ns_dev.add_task(dev.status)
ns_dev.add_task(dev.server)
ns_dev.add_task(dev.frontend)
ns.add_collection(ns_dev)

ns.add_collection(Collection.from_module(changelog))
ns.add_collection(Collection.from_module(deny))
ns.add_collection(Collection.from_module(deps))
ns.add_collection(Collection.from_module(docs))
ns.add_collection(Collection.from_module(git))
ns.add_collection(Collection.from_module(github))
ns.add_collection(Collection.from_module(marketplace))
ns.add_collection(Collection.from_module(measure))
ns.add_collection(Collection.from_module(notices))
ns.add_collection(Collection.from_module(public_api))
ns.add_collection(Collection.from_module(pup))
ns.add_collection(Collection.from_module(test))
ns.add_collection(Collection.from_module(version))

ns_vendor = Collection("vendor")
ns_vendor.add_task(vendor_commands.check_trust_anchors)
ns_vendor.add_task(vendor_commands.verify_upstream_inputs)
ns_vendor.add_task(vendor_commands.assemble_tree_artifacts)
ns_vendor.add_task(vendor_commands.smoke_runtime)
ns_vendor.add_task(vendor_commands.build_archive)
ns.add_collection(ns_vendor)

ns_format = Collection("format")
ns_format.add_collection(Collection.from_module(format_.scripts))
ns_format.add_collection(Collection.from_module(format_.build_system))
ns_format.add_collection(Collection.from_module(format_.server))
ns_format.add_collection(Collection.from_module(format_.frontend))
ns_format.add_collection(Collection.from_module(format_.cli))
ns.add_collection(ns_format)

ns_lint = Collection("lint")
ns_lint.add_collection(Collection.from_module(lint.scripts))
ns_lint.add_collection(Collection.from_module(lint.build_system))
ns_lint.add_collection(Collection.from_module(lint.server))
ns_lint.add_collection(Collection.from_module(lint.frontend))
ns_lint.add_collection(Collection.from_module(lint.cli))
ns_lint.add_collection(Collection.from_module(lint.workflows))
ns_lint.add_collection(Collection.from_module(lint.vendor_shims))
ns_lint.add_collection(Collection.from_module(lint.store_duplication))
ns_lint.add_collection(Collection.from_module(lint.skill_permissions))
ns_lint.add_collection(Collection.from_module(lint.bare_invocation))
ns_lint.add_collection(Collection.from_module(lint.claude_coupling))
ns_lint.add_collection(Collection.from_module(lint.dispatch_coherence))
ns_lint.add_collection(Collection.from_module(lint.integration_skills))
ns_lint.add_collection(Collection.from_module(lint.vcs_settings))
ns_lint.add_collection(Collection.from_module(lint.git_tokens))
ns_lint.add_collection(Collection.from_module(lint.skill_cli_refs))
ns.add_collection(ns_lint)

ns_types = Collection("types")
ns_types.add_collection(Collection.from_module(types.build_system))
ns_types.add_collection(Collection.from_module(types.frontend))
ns.add_collection(ns_types)
