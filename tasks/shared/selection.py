import dataclasses


@dataclasses.dataclass(frozen=True)
class Selection:
    """A test filter, rendered in each runner's own syntax.

    Every rendering is the runner's option alone, so a lane joins the parts
    of its command line itself and drops an empty one.
    """

    expression: str | None = None

    def __post_init__(self) -> None:
        if self.expression is not None and not self.expression.strip():
            raise ValueError("--filter needs an expression")

    @property
    def is_filtered(self) -> bool:
        return self.expression is not None

    def nextest(self) -> str:
        return self._option("-E")

    def pytest(self) -> str:
        return self._option("-k")

    def _option(self, name: str) -> str:
        if self.expression is None:
            return ""
        return f"{name} {_quoted(self.expression)}"

    @property
    def flag(self) -> str:
        return f"--filter {_quoted(self.expression or '')}"

    def nothing_selected(self, task: str) -> str:
        return (
            f"{self.flag} selected no tests; "
            f"see mise run {task} --help for its syntax"
        )


def _quoted(word: str) -> str:
    return "'" + word.replace("'", "'\"'\"'") + "'"
