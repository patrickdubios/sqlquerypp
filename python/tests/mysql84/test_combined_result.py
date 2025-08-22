from sqlquerypp import Compiler, MySQL84Compiler, Query

from ..common import CompilerTestCase


class CombinedResultTests(CompilerTestCase):
    def _get_compiler(self) -> Compiler:
        return MySQL84Compiler()

    def test_with_multiple_parameters_and_union_fragments(self) -> None:
        template = Query(
            """
            combined_result (SELECT col_a1 FROM table_a
                             WHERE criteria = %s) AS $id {
                SELECT a.col_a1, a.col_a2, b.col_b1, b.col_b2
                FROM table_a a
                INNER JOIN table_b b
                ON b.col_a1 = a.col_a1
                AND b.cond1 = %s
                AND b.cond2 = %s
                WHERE a.col_a1 = $id
            }
            UNION ALL
            combined_result (SELECT col_a1 FROM table_a
                             WHERE criteria = %s) AS $id {
                SELECT a.col_a1, a.col_a2, b.col_b1, b.col_b2
                FROM table_a a
                INNER JOIN table_b b
                ON b.col_a1 = a.col_a1
                AND b.cond3 = %s
                AND b.cond4 = %s
                WHERE a.col_a1 = $id
            }
            """,
            ["CRIT1", 1337, 42, "CRIT2", 31415, 1338],
        )
        expected = Query(
            self.loadQueryFromFile(
                __name__,
                "test_with_multiple_parameters_and_union_fragments",
            ),
            ["CRIT1", 1337, 42, 1337, 42, "CRIT2", 31415, 1338, 31415, 1338],
        )

        self.assertGeneratedQueryEqual(expected, template)
