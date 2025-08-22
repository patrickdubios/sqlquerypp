(WITH RECURSIVE all_entries (n, col_a1, col_a2, col_b1, col_b2) AS (
  WITH loop_values AS (
    SELECT
      col_a1
    FROM
      table_a
  )
  SELECT
    0,
    a.col_a1,
    a.col_a2,
    b.col_b1,
    b.col_b2
  FROM
    table_a AS a
    LEFT JOIN table_b AS b ON b.col_a1 = a.col_a1 AND b.cond1 = %s AND b.cond2 = %s
  WHERE
    a.col_a1 = (SELECT * FROM loop_values LIMIT 1)
  UNION ALL
  SELECT
    n + 1,
    a.col_a1,
    a.col_a2,
    b.col_b1,
    b.col_b2
  FROM
    all_entries
    LEFT JOIN table_a AS a ON a.col_a1 = (SELECT col_a1 FROM loop_values WHERE col_a1 > all_entries.col_a1 LIMIT 1)
    LEFT JOIN table_b AS b ON b.col_a1 = a.col_a1 AND b.cond1 = %s AND b.cond2 = %s
  WHERE
    n + 1 < (SELECT COUNT(*) FROM loop_values)
)
SELECT
  col_a1,
  col_a2,
  col_b1,
  col_b2
FROM
  all_entries
WHERE
  col_b1 IS NOT NULL AND col_b2 IS NOT NULL)
UNION ALL
(WITH RECURSIVE all_entries (n, col_a1, col_a2, col_b1, col_b2) AS (
  WITH loop_values AS (
    SELECT
      col_a1
    FROM
      table_a
  )
  SELECT
    0,
    a.col_a1,
    a.col_a2,
    b.col_b1,
    b.col_b2
  FROM
    table_a AS a
    LEFT JOIN table_b AS b ON b.col_a1 = a.col_a1 AND b.cond3 = %s AND b.cond4 = %s
  WHERE
    a.col_a1 = (SELECT * FROM loop_values LIMIT 1)
  UNION ALL
  SELECT
    n + 1,
    a.col_a1,
    a.col_a2,
    b.col_b1,
    b.col_b2
  FROM
    all_entries
    LEFT JOIN table_a AS a ON a.col_a1 = (SELECT col_a1 FROM loop_values WHERE col_a1 > all_entries.col_a1 LIMIT 1)
    LEFT JOIN table_b AS b ON b.col_a1 = a.col_a1 AND b.cond3 = %s AND b.cond4 = %s
  WHERE
    n + 1 < (SELECT COUNT(*) FROM loop_values)
)
SELECT
  col_a1,
  col_a2,
  col_b1,
  col_b2
FROM
  all_entries
WHERE
  col_b1 IS NOT NULL AND col_b2 IS NOT NULL)