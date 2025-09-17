from dataclasses import dataclass
from typing import Any, Sequence


@dataclass(frozen=True)
class Query:
    statement: str
    parameters: Sequence[Any]
