mod nodes;
mod state;
mod types;

pub use {
    nodes::Node,
    state::{FinalParserState, ParserState},
};

#[cfg(test)]
mod tests {
    use crate::parser::{nodes::CompleteCombinedResultNode, ParserState};

    fn get_combined_result_nodes(query: &str)
                                 -> Vec<CompleteCombinedResultNode> {
        let owned = query.to_string();
        let mut parser = ParserState::initialize(&owned);
        parser.parse().unwrap();

        let finalized = parser.finalize().unwrap();
        finalized.combined_result_nodes.clone()
    }

    #[test]
    fn no_nodes_from_empty_string() {
        let nodes = get_combined_result_nodes("");
        assert_eq!(0, nodes.len());
    }

    #[test]
    fn no_nodes_from_sql_query() {
        let nodes = get_combined_result_nodes("SELECT * FROM somewhere;");
        assert_eq!(0, nodes.len());
    }

    #[test]
    fn node_found() {
        let query = "
            SELECT * FROM
            (
                combined_result (SELECT col_a1 FROM table_a) AS $id_a {
                    SELECT a.col_a1, a.col_a2, b.col_b1, b.col_b2 FROM table_a a
                    INNER JOIN table_b b
                    ON b.col_a1 = a.col_a1 AND b.cond1 = %s AND b.cond2 = %s
                    WHERE a.col_a1 = $id_a
                }
            )
        ";
        let nodes = get_combined_result_nodes(query);
        assert_eq!(
            vec![
                CompleteCombinedResultNode::new(
                    57,
                    371,
                    "SELECT col_a1 FROM table_a".to_string(),
                    "$id_a".to_string(),
                    111,
                    "SELECT a.col_a1, a.col_a2, b.col_b1, b.col_b2 FROM table_a a
                     INNER JOIN table_b b
                     ON b.col_a1 = a.col_a1 AND b.cond1 = %s AND b.cond2 = %s
                     WHERE a.col_a1 = $id_a".to_string(),
                ),
            ],
            nodes,
        );
    }

    #[test]
    fn nodes_found() {
        let query = "
            SELECT * FROM
            (
                combined_result (SELECT col_a1 FROM table_a) AS $id_a {
                    SELECT a.col_a1, a.col_a2, b.col_b1, b.col_b2 FROM table_a a
                    INNER JOIN table_b b
                    ON b.col_a1 = a.col_a1 AND b.cond1 = %s AND b.cond2 = %s
                    WHERE a.col_a1 = $id_a
                }
                UNION ALL
                combined_result (SELECT col_z1 FROM table_z) AS $id_z {
                    SELECT z.col_z1, z.col_z2, b.col_b1, b.col_b2 FROM table_z z
                    INNER JOIN table_b b
                    ON b.col_z1 = z.col_z1 AND b.cond3 = ? AND b.cond4 = ?
                    WHERE z.col_z1 = $id_z
                }
            )
        ";
        let nodes = get_combined_result_nodes(query);
        assert_eq!(
            vec![
                CompleteCombinedResultNode::new(
                    57,
                    371,
                    "SELECT col_a1 FROM table_a".to_string(),
                    "$id_a".to_string(),
                    111,
                    "SELECT a.col_a1, a.col_a2, b.col_b1, b.col_b2 FROM table_a a
                    INNER JOIN table_b b
                    ON b.col_a1 = a.col_a1 AND b.cond1 = %s AND b.cond2 = %s
                    WHERE a.col_a1 = $id_a".to_string()
                ),
                CompleteCombinedResultNode::new(
                    415,
                    727,
                    "SELECT col_z1 FROM table_z".to_string(),
                    "$id_z".to_string(),
                    469,
                    "SELECT z.col_z1, z.col_z2, b.col_b1, b.col_b2 FROM table_z z
                    INNER JOIN table_b b
                    ON b.col_z1 = z.col_z1 AND b.cond3 = ? AND b.cond4 = ?
                    WHERE z.col_z1 = $id_z".to_string()
                ),
            ],
            nodes,
        );
    }
}
