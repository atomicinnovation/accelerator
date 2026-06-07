from invoke import Collection

from . import (
    build,
    changelog,
    deps,
    dev,
    git,
    github,
    lint,
    marketplace,
    release,
    test,
    version
)
from . import format as format_  # `format` shadows a builtin; alias the import

ns = Collection()

ns_prerelease = Collection("prerelease")
ns_prerelease.add_task(release.prerelease, default=True)
ns_prerelease.add_task(release.prerelease_prepare, name="prepare")
ns_prerelease.add_task(release.prerelease_finalise, name="finalise")
ns.add_collection(ns_prerelease)

ns_release = Collection("release")
ns_release.add_task(release.release, default=True)
ns_release.add_task(release.release_prepare, name="prepare")
ns_release.add_task(release.release_finalise, name="finalise")
ns.add_collection(ns_release)

ns.add_collection(Collection.from_module(build))

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
ns.add_collection(Collection.from_module(deps))
ns.add_collection(Collection.from_module(git))
ns.add_collection(Collection.from_module(github))
ns.add_collection(Collection.from_module(marketplace))
ns.add_collection(Collection.from_module(test))
ns.add_collection(Collection.from_module(version))

ns_format = Collection("format")
ns_format.add_collection(Collection.from_module(format_.scripts))  # format.scripts.check / .fix
ns.add_collection(ns_format)

ns_lint = Collection("lint")
ns_lint.add_collection(Collection.from_module(lint.scripts))  # lint.scripts.shellcheck / .bashisms
ns.add_collection(ns_lint)
