from abc import ABC, abstractmethod
import os
from textwrap import dedent
from typing import Any, Sequence
from unittest import TestCase

from sqlquerypp import Compiler, Query


class CompilerTestCase(TestCase, ABC):
    maxDiff = None

    @abstractmethod
    def _get_compiler(self) -> Compiler:
        pass

    def loadQueryFromFile(
        self, test_module_name: str, test_function_name: str
    ) -> str:
        # tests.mysql84.test_something -> mysql84/test_something
        module_subpath = "/".join(test_module_name.split(".")[1:])
        final_path = os.path.join(
            os.path.dirname(__file__),
            "expected_queries",
            module_subpath,
            f"{test_function_name}.sql",
        )
        with open(final_path, "r") as fp:
            return fp.read()

    def assertGeneratedQueryEqual(
        self,
        expected: Query,
        template: Query,
    ) -> None:
        expected = self._normalize_query(expected)
        actual_query = self._normalize_query(
            self._get_compiler().compile(template)
        )
        self._assert_for_equal_statements(expected, actual_query)
        self.assertEqual(expected.parameters, actual_query.parameters)

    def _assert_for_equal_statements(
        self, expected: Query, actual_query: Query
    ) -> None:
        msg = f"resulting query is: {actual_query.statement}"
        self.assertEqual(expected.statement, actual_query.statement, msg)

    def _normalize_query(self, query: Query) -> Query:
        return Query(
            statement=dedent(query.statement.strip()),
            parameters=query.parameters,
        )
