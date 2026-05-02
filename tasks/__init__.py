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

ns.add_task(release.prerelease, name="prerelease")
ns.add_task(release.release, name="release")
ns.add_task(release.prerelease_prepare, name="prerelease-prepare")
ns.add_task(release.prerelease_finalize, name="prerelease-finalize")
ns.add_task(release.stable_prepare, name="stable-prepare")
ns.add_task(release.stable_publish, name="stable-publish")
ns.add_task(release.post_stable_prepare, name="post-stable-prepare")
ns.add_task(release.post_stable_publish, name="post-stable-publish")

ns.add_collection(Collection.from_module(build))
ns.add_collection(Collection.from_module(changelog))
ns.add_collection(Collection.from_module(deps))
ns.add_collection(Collection.from_module(git))
ns.add_collection(Collection.from_module(github))
ns.add_collection(Collection.from_module(marketplace))
ns.add_collection(Collection.from_module(test))
ns.add_collection(Collection.from_module(version))
