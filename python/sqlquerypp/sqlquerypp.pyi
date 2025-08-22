class CombinedResultNodeSlice:
    scope_begin: int
    scope_end: int

class CompiledQueryDescriptor:
    statement: str
    combined_result_node_slices: list[CombinedResultNodeSlice]

    def __init__(
        self,
        statement: str,
        combined_result_node_slices: list[CombinedResultNodeSlice],
    ): ...

def preprocess_mysql84_query(statement: str) -> CompiledQueryDescriptor: ...
