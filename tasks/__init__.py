from invoke import Collection

from . import (
    test,
)

ns = Collection()

ns.add_collection(Collection.from_module(test))
