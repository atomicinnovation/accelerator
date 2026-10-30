import itertools
import re
from pathlib import Path

import pytest

from tasks.eval.citing_once.rule import (
    citation_order,
    cite_once,
    key_order,
    merged_tier,
    normalised_url,
    same_source,
    sources,
)
from tasks.eval.citing_once.sources import Citation, TierText, parse_sources

REPO_ROOT = Path(__file__).resolve().parents[3]
RULE = REPO_ROOT / "skills/research/outputters/citing-once/SKILL.md"


def _rule() -> str:
    return RULE.read_text()


def _section(heading: str) -> str:
    text = _rule()
    start = text.index(f"## {heading}\n")
    following = text.find("\n## ", start + 1)
    return text[start : following if following != -1 else None]


def _url_table() -> list[tuple[str, str, str]]:
    return re.findall(
        r"^\| `([^`]+)` \| `([^`]+)` \| (one|two) \|$",
        _section("Same source"),
        re.MULTILINE,
    )


def _worked_example() -> tuple[str, str]:
    citations, merged = re.findall(
        r"```text\n(.*?)```", _section("Worked example"), re.DOTALL
    )
    return citations, merged


def _cite(
    url: str,
    tier: str = "tier-2",
    venue: str | None = "example.org",
    title: str = "Title",
    block: tuple[tuple[str, str], ...] = (),
) -> Citation:
    return Citation(
        title=title,
        url=url,
        tier=TierText.parse(tier),
        venue=venue,
        block=block,
    )


def test_a_sources_entry_parses_into_its_parts() -> None:
    (citation,) = parse_sources(
        "## Sources\n"
        "- [The Notifier](https://x.example/a) — tier-3 (retracted; also "
        "tier-1) — Example Journal\n"
        "  - doi: 10.x/y\n"
        "  - published: 2019\n"
    )
    assert citation == Citation(
        title="The Notifier",
        url="https://x.example/a",
        tier=TierText(base=3, marks=("retracted",), also=(1,)),
        venue="Example Journal",
        block=(("doi", "10.x/y"), ("published", "2019")),
    )


def test_an_entry_renders_as_it_was_written() -> None:
    entry = (
        "- [A](x.example/a) — tier-3 (retracted; withdrawn; also tier-2, "
        "tier-1) — x.example\n  - doi: 10.X/Y\n"
    )
    (citation,) = parse_sources("## Sources\n" + entry)
    assert citation.render() == entry


def test_an_entry_may_have_no_venue() -> None:
    (citation,) = parse_sources("## Sources\n- [A](x.example/a) — tier-3\n")
    assert citation.venue is None
    assert citation.render() == "- [A](x.example/a) — tier-3\n"


def test_none_found_cites_nothing() -> None:
    assert parse_sources("## Sources\nNone found.\n") == []


def test_only_the_sources_section_is_read() -> None:
    document = (
        "## Findings\n- [Not](a.example/n) — tier-1 — a.example\n"
        "## Sources\n- [Yes](b.example/y) — tier-2 — b.example\n"
        "## Open Threads\n- [No](c.example/n) — tier-1 — c.example\n"
    )
    assert [citation.title for citation in parse_sources(document)] == ["Yes"]


def test_the_rule_carries_its_url_table() -> None:
    assert len(_url_table()) == 5


@pytest.mark.parametrize(("first", "second", "verdict"), _url_table())
def test_the_url_table_holds(first: str, second: str, verdict: str) -> None:
    assert same_source(_cite(first), _cite(second)) == (verdict == "one")


def test_normalisation_keeps_the_query_and_the_paths_case() -> None:
    assert normalised_url("https://WWW.Ex.org/A/?q=B#f") == "ex.org/A?q=B"


def test_a_doi_url_and_a_doi_key_carry_one_doi() -> None:
    assert same_source(
        _cite("https://dx.doi.org/10.X/Y"),
        _cite("pub.example/p", block=(("doi", "10.x/y"),)),
    )


def test_a_doi_is_percent_decoded_from_its_url() -> None:
    assert same_source(
        _cite("doi.org/10.1016/S0140-6736%2820%2930183-5"),
        _cite(
            "pub.example/p", block=(("doi", "10.1016/s0140-6736(20)30183-5"),)
        ),
    )


def test_matches_chain_into_one_source() -> None:
    chain = [
        _cite("doi.org/10.x/y", "tier-1"),
        _cite("pub.example/p", block=(("doi", "10.x/y"),)),
        _cite("https://www.pub.example/p/", "tier-3"),
        _cite("other.example/q"),
    ]
    assert [len(source) for source in sources(chain)] == [3, 1]


@pytest.mark.parametrize(
    ("tiers", "merged"),
    [
        (["tier-2", "tier-2"], "tier-2"),
        (["tier-1", "tier-2", "tier-3"], "tier-3 (also tier-2, tier-1)"),
        (["tier-2", "tier-3 (also tier-1)"], "tier-3 (also tier-2, tier-1)"),
        (["tier-1", "tier-3 (retracted)"], "tier-3 (retracted; also tier-1)"),
        (
            [
                "tier-3 (withdrawn; also tier-2)",
                "tier-3 (retracted; also tier-1)",
            ],
            "tier-3 (retracted; withdrawn; also tier-2, tier-1)",
        ),
    ],
)
def test_the_merge_rule(tiers: list[str], merged: str) -> None:
    citations = [_cite("x.example/a", tier) for tier in tiers]
    assert str(merged_tier(citations)) == merged


@pytest.mark.parametrize(
    ("first", "second"),
    [
        ("Example Journal", "example.org"),
        ("a.example.org", "b.example.org"),
        ("acta Journal", "Zeta Journal"),
        ("Example Journal", "example journal"),
        ("GitHub", None),
    ],
)
def test_venues_order_as_the_rule_says(first: str, second: str | None) -> None:
    earlier = _cite("x.example/a", venue=first)
    later = _cite("x.example/a", venue=second)
    assert citation_order([later, earlier]) == [earlier, later]


@pytest.mark.parametrize(
    ("first", "second"),
    [
        ("https://brandur.org/notifier", "brandur.org/notifier/"),
        ("https://brandur.org/notifier", "https://www.brandur.org/notifier/"),
        ("http://example.org/a", "example.org/a"),
    ],
)
def test_urls_order_as_the_rule_says(first: str, second: str) -> None:
    earlier = _cite(first)
    later = _cite(second)
    assert citation_order([later, earlier]) == [earlier, later]


def test_tier_orders_before_venue() -> None:
    weaker = _cite("x.example/a", "tier-3", venue="example.org")
    stronger = _cite("x.example/a", "tier-1", venue="Example Journal")
    assert citation_order([stronger, weaker]) == [weaker, stronger]


def test_a_full_tie_takes_the_first_title_and_value() -> None:
    (merged,) = cite_once(
        [
            _cite("b.example/n", title="notifier", block=(("accessed", "2"),)),
            _cite("b.example/n", title="Notifier", block=(("accessed", "1"),)),
        ],
        key_order(_rule()),
    )
    assert (merged.title, merged.block) == ("Notifier", (("accessed", "1"),))


def test_each_key_comes_from_its_first_carrier() -> None:
    (merged,) = cite_once(
        [
            _cite("p.example/a", "tier-3", block=(("openalex_id", "W1"),)),
            _cite(
                "p.example/a",
                "tier-1",
                block=(("doi", "10.x/y"), ("openalex_id", "W2")),
            ),
        ],
        key_order(_rule()),
    )
    assert merged.block == (("doi", "10.x/y"), ("openalex_id", "W1"))


def test_unrecommended_keys_follow_byte_wise() -> None:
    (merged,) = cite_once(
        [
            _cite(
                "p.example/a",
                block=(("zeta", "1"), ("doi", "10.x/y"), ("alpha", "2")),
            )
        ],
        key_order(_rule()),
    )
    assert [key for key, _ in merged.block] == ["doi", "alpha", "zeta"]


def test_the_key_order_is_the_rules_table() -> None:
    order = key_order(_rule())
    assert order[:3] == ("doi", "arxiv_id", "openalex_id")
    assert order[-1] == "pdf_url"


def test_the_worked_example_holds() -> None:
    citations, merged = _worked_example()
    assert cite_once(
        parse_sources("## Sources\n" + citations), key_order(_rule())
    ) == parse_sources("## Sources\n" + merged)


def test_citing_once_ignores_input_order() -> None:
    citations, _ = _worked_example()
    parsed = parse_sources("## Sources\n" + citations)
    results = {
        tuple(cite_once(list(order), key_order(_rule())))
        for order in itertools.permutations(parsed)
    }
    assert len(results) == 1
