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
        if self.expression is None:
            return ""
        return f"-E {_quoted(self.expression)}"

    def nothing_selected(self, task: str) -> str:
        return (
            f"--filter {_quoted(self.expression or '')} selected no "
            f"tests; see mise run {task} --help for its syntax"
        )


def _quoted(word: str) -> str:
    return "'" + word.replace("'", "'\"'\"'") + "'"
