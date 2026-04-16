from invoke import Collection

from . import (
    changelog,
    git,
    marketplace,
    release,
    test,
    version
)

ns = Collection()

ns.add_task(release.prerelease, name="prerelease")
ns.add_task(release.release, name="release")

ns.add_collection(Collection.from_module(changelog))
ns.add_collection(Collection.from_module(marketplace))
ns.add_collection(Collection.from_module(git))
ns.add_collection(Collection.from_module(test))
ns.add_collection(Collection.from_module(version))
