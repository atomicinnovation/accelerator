"""Judge fixtures and written Sources against the rule's prediction."""

import itertools
from collections import Counter
from collections.abc import Sequence

from tasks.eval.citing_once.fixtures import (
    Composition,
    Criterion,
    Expectation,
)
from tasks.eval.citing_once.rule import (
    carried_dois,
    cite_once,
    normalised_url,
)
from tasks.eval.citing_once.sources import Citation


def predict(
    composition: Composition,
    order: Sequence[str],
    recommended: Sequence[str],
) -> list[Citation]:
    citations = [
        citation
        for document in order
        for citation in composition.citations_in(document)
    ]
    return cite_once(citations, recommended)


def _identities(criterion: Criterion) -> set[str]:
    return {
        identity
        for placed in criterion.citations
        for identity in (
            normalised_url(placed.citation.url),
            *carried_dois(placed.citation),
        )
    }


def isolation_conflicts(composition: Composition) -> list[str]:
    conflicts = [
        f"{first.id} and {second.id} share a source"
        for first, second in itertools.combinations(composition.criteria, 2)
        if _identities(first) & _identities(second)
    ]
    for criterion in composition.criteria:
        for placed in criterion.citations:
            selecting = [
                other.id
                for other in composition.criteria
                if other.selects(placed.citation)
            ]
            if selecting != [criterion.id]:
                conflicts.append(
                    f"{criterion.id}'s {placed.citation.url} is selected by "
                    f"{selecting or 'no criterion'}"
                )
    return conflicts


def _met(expect: Sequence[Expectation], entries: Sequence[Citation]) -> bool:
    return any(
        all(
            wanted.met_by(entry)
            for wanted, entry in zip(expect, arranged, strict=True)
        )
        for arranged in itertools.permutations(entries)
    )


def criterion_failures(
    composition: Composition, entries: Sequence[Citation]
) -> list[str]:
    failures = []
    for criterion in composition.criteria:
        selected = [entry for entry in entries if criterion.selects(entry)]
        if len(selected) != len(criterion.expect) or not _met(
            criterion.expect, selected
        ):
            written = "; ".join(entry.render().strip() for entry in selected)
            failures.append(
                f"{criterion.id}: {criterion.statement}; wrote {written}"
            )
    return failures


def entry_differences(
    predicted: Sequence[Citation], written: Sequence[Citation]
) -> list[str]:
    expected = Counter(entry.render() for entry in predicted)
    actual = Counter(entry.render() for entry in written)
    missing = expected - actual
    unexpected = actual - expected
    return [f"missing {entry.strip()}" for entry in missing.elements()] + [
        f"unexpected {entry.strip()}" for entry in unexpected.elements()
    ]
