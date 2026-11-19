from typing import override

_SUBJECT = "shared VCS fixtures"


class SharedFixturesError(Exception):
    def __init__(self, *problems: str, failure: str | None = None) -> None:
        super().__init__(*problems)
        self.problems = problems
        self.failure = failure

    @override
    def __str__(self) -> str:
        return "\n".join(
            [
                *([self.failure] if self.failure else []),
                *(f"{_SUBJECT}: {problem}" for problem in self.problems),
            ]
        )
