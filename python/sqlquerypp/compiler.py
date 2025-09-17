from abc import ABC, abstractmethod
import hashlib
import re
from typing import Any, Sequence

from .sqlquerypp import (
    CompiledQueryDescriptor,
    preprocess_mysql84_query,
)
from .types import Query


class Compiler(ABC):
    @abstractmethod
    def _compile_template(self, statement: str) -> CompiledQueryDescriptor:
        pass

    def __init__(self, variable_placeholder: str = "?") -> None:
        self._cache: dict[str, CompiledQueryDescriptor] = {}
        self._variable_placeholder = variable_placeholder

    def compile(self, template: Query) -> Query:
        """
        Compiles a given query to valid SQL.
        """
        descriptor = self._resolve_compiled_descriptor(template.statement)
        parameters = self._resolve_parameters_from_descriptor(
            template, descriptor
        )
        return Query(statement=descriptor.statement, parameters=parameters)

    def _resolve_parameters_from_descriptor(
        self,
        template: Query,
        descriptor: CompiledQueryDescriptor,
    ) -> Sequence[Any]:
        final_parameters: list[Any] = []

        last_statement_offset = 0
        last_parameters_offset = 0
        for slice in descriptor.combined_result_node_slices:
            parameters_outside_combined_result = template.statement[
                last_statement_offset : slice.scope_begin
            ].count(self._variable_placeholder)
            parameters_within_combined_result = template.statement[
                slice.scope_begin : slice.scope_end
            ].count(self._variable_placeholder)

            final_parameters += template.parameters[
                last_parameters_offset : last_parameters_offset
                + parameters_outside_combined_result
            ]
            last_parameters_offset += parameters_outside_combined_result

            # compiler needs to duplicate params within combined_result nodes.
            # that's why we append them twice, but each in order.
            for _ in range(2):
                final_parameters += template.parameters[
                    last_parameters_offset : last_parameters_offset
                    + parameters_within_combined_result
                ]
            last_parameters_offset += parameters_within_combined_result
            last_statement_offset = slice.scope_end

        if last_statement_offset < len(template.statement):
            final_parameters += template.parameters[last_parameters_offset:]

        return final_parameters

    def _resolve_compiled_descriptor(
        self, statement: str
    ) -> CompiledQueryDescriptor:
        key = self._build_cache_key(statement)
        if key not in self._cache:
            self._cache[key] = self._compile_template(statement)
        return self._cache[key]

    def _build_cache_key(self, statement: str) -> str:
        normalized = self._get_normalized_query_template_string(statement)
        cache_key = hashlib.sha256(normalized).hexdigest()
        return cache_key

    def _get_normalized_query_template_string(self, statement: str) -> bytes:
        without_new_lines = self._strip_new_lines(statement)
        cleaned = re.sub(r"[ ]+", " ", without_new_lines)
        encoded = cleaned.encode()
        return encoded

    def _strip_new_lines(self, query: str) -> str:
        without_cr = query.replace("\r", " ")
        without_lf = without_cr.replace("\n", " ")
        return without_lf


class MySQL84Compiler(Compiler):
    """
    An implementation compiling `sqlquerypp` specific syntax to valid MySQL 8.4
    queries.
    """

    def _compile_template(self, statement: str) -> CompiledQueryDescriptor:
        if self.pep_249_placeholders:
            statement = statement.replace("%s", "?")

        result = preprocess_mysql84_query(statement)

        if self.pep_249_placeholders:
            return CompiledQueryDescriptor(
                statement=result.statement.replace("?", "%s"),
                combined_result_node_slices=result.combined_result_node_slices,
            )
        return result

    def __init__(self, pep_249_placeholders: bool = True) -> None:
        self.pep_249_placeholders = pep_249_placeholders
        super().__init__("%s" if self.pep_249_placeholders else "?")
