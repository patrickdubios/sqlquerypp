# sqlquerypp: SQL query preprocessor

`sqlquerypp` is a library for preprocessing SQL queries. Main purpose
is writing highly optimized queries with a simplified syntax, allowing
for both maintainability and high performance.

## Limitations

Currently, only MySQL 8.4 syntax is supported.

## Why preprocessing SQL queries?

SQL (Structed Query Language) follows a declarative paradigm, i.e. a query
explains "what should be done" not "how should it be done". This stands in
contrast to imperative programming, which expresses the "how should a
certain task be fulfilled" aspect.

Database systems' internals are responsible for maintaining this aspect.
But, however, for certain and large data structures, writing down "naive"
queries sometimes result in poor performance.

## Supported performance optimizations

### Combined `UNION` queries

Consider the following original query:

  ```
  SELECT entity_b.*
  FROM entity_b
  INNER JOIN entity_a
    ON entity_a.id = entity_b.entity_a_id
    AND entity_a.criteria = 1337;
  ```

This is a very simplified example, but if you assume `entity_b` contains very
many items, even correct index conditions may exhaust any DBMS' join buffer.

An alternative approach might be doing a loop at application side (Python
pseudocode), if network overhead is acceptable:

  ```
  all_matches_in_entity_b = []
  for entity_a_id in [rec.id
                      for rec in mysql_query("SELECT id FROM entity_a "
                                             "WHERE criteria = 1337")]:
      inner_result = mysql_query("SELECT * FROM entity_b "
                                 f"WHERE entity_a_id = {entity_a_id}")
      all_matches_in_entity_b += inner_result
  ```

The following statement, being no valid SQL, translates to a MySQL
native construct of `Recursive Common Table Expression` and `UNION`
fragments when being compiled by `sqlquerypp`. This allows for maximal
query performance, because the inner query with reduced complexity
is still taken into account. At the same time, it grants minimal I/O
overhead as only one query is executed on the database:

  ```
  combined_result (SELECT id FROM entity_a WHERE criteria = 1337) AS $id {
      SELECT * FROM entity_b WHERE entity_a_id = $id;
  }
  ```