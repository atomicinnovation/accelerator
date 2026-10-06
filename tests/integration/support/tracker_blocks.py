"""Tracker pull and push blocks shared by the dump and parsing cases.

Each block is indented to sit under `<tracker>:` in a config frontmatter.
"""

from dataclasses import dataclass

TRACKERS = ("jira", "linear")

_NOUNS = {"jira": "projects", "linear": "teams"}
_OTHER_NOUN = {"jira": "teams", "linear": "projects"}


@dataclass(frozen=True)
class Blocks:
    pull: str | None = None
    push: str | None = None

    def frontmatter(self, tracker: str, settings: str = "") -> str:
        sections = [f"  {name}:{body}" for name, body in self._set()]
        body = settings + "".join(sections)
        tracker_block = f"{tracker}:\n{body}" if body else ""
        return f"---\nwork:\n  integration: {tracker}\n{tracker_block}---\n"

    def _set(self) -> list[tuple[str, str]]:
        return [
            (name, body)
            for name, body in (("pull", self.pull), ("push", self.push))
            if body is not None
        ]


def pull_cases(tracker: str) -> dict[str, Blocks]:
    noun = _NOUNS[tracker]
    other = _OTHER_NOUN[tracker]
    return {
        "unset": Blocks(),
        "valid": Blocks(
            pull=(
                f"\n    additional_{noun}: [core, ops]\n"
                "    filters:\n      label: [bug]\n      state: [open]\n"
                "    max_items: 3\n"
                "    max_pages:\n      discovery: 2\n"
            ),
            push="\n    max_items: unlimited\n",
        ),
        "all-scope": Blocks(pull=f"\n    all_{noun}: true\n"),
        "empty-mapping": Blocks(pull=" {}\n"),
        "non-mapping": Blocks(pull=" [a, b]\n"),
        "scalar": Blocks(pull=" yes\n"),
        "filters-not-a-mapping": Blocks(pull="\n    filters: [label]\n"),
        "unsupported-filter": Blocks(
            pull="\n    filters:\n      colour: [red]\n"
        ),
        "reserved-filter": Blocks(pull="\n    filters:\n      any: [x]\n"),
        "max-pages-zero": Blocks(pull="\n    max_pages: 0\n"),
        "max-items-negative": Blocks(pull="\n    max_items: -1\n"),
        "unknown-max-pages-sub": Blocks(
            pull="\n    max_pages:\n      bogus: 2\n"
        ),
        "all-and-additional": Blocks(
            pull=f"\n    all_{noun}: true\n    additional_{noun}: [core]\n"
        ),
        "unknown-key": Blocks(pull="\n    bogus: 1\n"),
        "wrong-noun": Blocks(pull=f"\n    additional_{other}: [core]\n"),
        "bad-push-ceiling": Blocks(push="\n    max_items: lots\n"),
        "push-unknown-key": Blocks(push="\n    bogus: 1\n"),
        "two-unknown-keys": Blocks(
            pull="\n    bogus: 1\n    max_pages:\n      nope: 2\n"
        ),
        "sub-key-before-top-level-key": Blocks(
            pull="\n    max_pages:\n      nope: 2\n    bogus: 1\n"
        ),
        "unsupported-before-reserved-filter": Blocks(
            pull="\n    filters:\n      colour: [red]\n      any: [x]\n"
        ),
        "reserved-before-unsupported-filter": Blocks(
            pull="\n    filters:\n      any: [x]\n      colour: [red]\n"
        ),
        "unknown-key-and-bad-ceiling": Blocks(
            pull="\n    max_items: lots\n    bogus: 1\n"
        ),
    }


def block_cases() -> list[tuple[str, str, Blocks]]:
    return [
        (tracker, name, blocks)
        for tracker in TRACKERS
        for name, blocks in pull_cases(tracker).items()
    ]


def block_case_id(case: tuple[str, str, Blocks]) -> str:
    tracker, name, _ = case
    return f"{tracker}-{name}"
