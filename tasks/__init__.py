from invoke import Collection

from . import (
    build,
    changelog,
    deps,
    git,
    marketplace,
    release,
    test,
    version
)

ns = Collection()

ns.add_task(release.prerelease, name="prerelease")
ns.add_task(release.release, name="release")

ns.add_collection(Collection.from_module(build))
ns.add_collection(Collection.from_module(deps))
ns.add_collection(Collection.from_module(changelog))
ns.add_collection(Collection.from_module(marketplace))
ns.add_collection(Collection.from_module(git))
ns.add_collection(Collection.from_module(test))
ns.add_collection(Collection.from_module(version))
