"""A reading of the citing-once rule, independent of any model.

It predicts the Sources entries a composer or ``synthesise`` should write,
so a session's output can be checked entry by entry.
"""

import re
from collections.abc import Iterable, Sequence
from urllib.parse import unquote

from tasks.eval.citing_once.sources import Citation, TierText

SCHEME = re.compile(r"^[A-Za-z][A-Za-z0-9+.-]*://")
BARE_DOMAIN = re.compile(r"^[A-Za-z0-9-]+(?:\.[A-Za-z0-9-]+)+$")
DOI_HOSTS = frozenset({"doi.org", "dx.doi.org"})
RETRACTION_MARKS = ("retracted", "withdrawn")


def _host_and_rest(url: str) -> tuple[str, str]:
    bare = SCHEME.sub("", url).split("#", 1)[0]
    cut = min(
        (at for at in (bare.find("/"), bare.find("?")) if at != -1),
        default=len(bare),
    )
    return bare[:cut], bare[cut:]


def normalised_url(url: str) -> str:
    host, rest = _host_and_rest(url)
    path, question, query = rest.partition("?")
    host = host.lower().removeprefix("www.")
    return host + path.removesuffix("/") + question + query


def carried_dois(citation: Citation) -> frozenset[str]:
    dois = {value.lower() for key, value in citation.block if key == "doi"}
    normalised = normalised_url(citation.url)
    host, _, path = normalised.partition("/")
    if host in DOI_HOSTS and path:
        dois.add(unquote(path.split("?", 1)[0]).lower())
    return frozenset(dois)


def same_source(first: Citation, second: Citation) -> bool:
    """Whether two citations match directly, before any chaining."""
    return normalised_url(first.url) == normalised_url(second.url) or bool(
        carried_dois(first) & carried_dois(second)
    )


def sources(citations: Sequence[Citation]) -> list[list[Citation]]:
    """Group citations into sources, chaining DOI and URL matches."""
    groups: list[list[Citation]] = []
    for citation in citations:
        linked = [g for g in groups if any(same_source(citation, c) for c in g)]
        merged = [c for group in linked for c in group] + [citation]
        unlinked = [g for g in groups if all(g is not lk for lk in linked)]
        groups = [*unlinked, merged]
    return groups


def merged_tier(citations: Iterable[Citation]) -> TierText:
    tiers = {tier for citation in citations for tier in citation.tier.tiers}
    marks = {mark for citation in citations for mark in citation.tier.marks}
    unknown = marks - set(RETRACTION_MARKS)
    if unknown:
        raise ValueError(f"the rule names no such mark: {sorted(unknown)}")
    weakest = max(tiers)
    return TierText(
        base=weakest,
        marks=tuple(mark for mark in RETRACTION_MARKS if mark in marks),
        also=tuple(sorted(tiers - {weakest}, reverse=True)),
    )


def _folded(text: str) -> tuple[str, bytes]:
    return text.casefold(), text.encode()


def _venue_key(venue: str | None) -> tuple[int, tuple[str, bytes]]:
    if venue is None:
        return 2, ("", b"")
    return (1 if BARE_DOMAIN.match(venue) else 0), _folded(venue)


def _url_key(url: str) -> tuple[int, bytes]:
    for rank, scheme in enumerate(("https://", "http://")):
        if url.startswith(scheme):
            return rank, url.encode()
    return 2, url.encode()


def _order_key(citation: Citation) -> tuple[object, ...]:
    return (
        -citation.tier.base,
        _venue_key(citation.venue),
        _url_key(citation.url),
    )


def citation_order(citations: Iterable[Citation]) -> list[Citation]:
    return sorted(citations, key=_order_key)


def _fully_tied(first: Citation, second: Citation) -> bool:
    return (first.tier.base, first.venue, first.url) == (
        second.tier.base,
        second.venue,
        second.url,
    )


def _first_value(carriers: Sequence[Citation], key: str) -> str:
    tied = [c for c in carriers if _fully_tied(c, carriers[0])]
    return min(
        (value for c in tied for k, value in c.block if k == key),
        key=_folded,
    )


def key_order(rule: str) -> tuple[str, ...]:
    section = rule.split("## Block order\n", 1)[1].split("\n## ", 1)[0]
    return tuple(re.findall(r"^\| `([a-z_]+)` \|$", section, re.MULTILINE))


def _block_key(key: str, recommended: Sequence[str]) -> tuple[int, bytes]:
    if key in recommended:
        return recommended.index(key), b""
    return len(recommended), key.encode()


def merged_entry(
    citations: Sequence[Citation], recommended: Sequence[str]
) -> Citation:
    ordered = citation_order(citations)
    representative = ordered[0]
    title = min(
        (c.title for c in ordered if _fully_tied(c, representative)),
        key=_folded,
    )
    keys = {key for citation in ordered for key, _ in citation.block}
    block = tuple(
        (
            key,
            _first_value(
                [c for c in ordered if key in dict(c.block)],
                key,
            ),
        )
        for key in sorted(keys, key=lambda k: _block_key(k, recommended))
    )
    return Citation(
        title=title,
        url=representative.url,
        tier=merged_tier(citations),
        venue=representative.venue,
        block=block,
    )


def cite_once(
    citations: Sequence[Citation], recommended: Sequence[str]
) -> list[Citation]:
    """One merged entry per source, in a stable order for comparison."""
    entries = [merged_entry(group, recommended) for group in sources(citations)]
    return sorted(entries, key=lambda entry: entry.render())
