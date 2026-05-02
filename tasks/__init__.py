from invoke import Collection

from . import (
    build,
    changelog,
    deps,
    git,
    github,
    marketplace,
    release,
    test,
    version
)

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
ns.add_collection(Collection.from_module(changelog))
ns.add_collection(Collection.from_module(deps))
ns.add_collection(Collection.from_module(git))
ns.add_collection(Collection.from_module(github))
ns.add_collection(Collection.from_module(marketplace))
ns.add_collection(Collection.from_module(test))
ns.add_collection(Collection.from_module(version))
