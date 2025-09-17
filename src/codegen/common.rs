use {
    crate::error::QueryCompilerError,
    sqlparser::{dialect::GenericDialect, parser::Parser},
};

/// Reformats (i.e. indents and normalizes) a given SQL string to make
/// it more human-readable.
///
/// This also ensures the query is valid SQL as far the `sqlparser`
/// crate can tell. In case the passed SQL string is invalid, an
/// according error is returned.
pub fn format_query_prettily(query: &str)
                             -> Result<String, QueryCompilerError> {
    let parser = Parser::new(&GenericDialect {});
    let parsed =
        parser.try_with_sql(query)
              .map_err(|e| {
                  QueryCompilerError::ResultingQueryInvalid(query.into(), e)
              })?
              .parse_query()
              .map_err(|e| {
                  QueryCompilerError::ResultingQueryInvalid(query.into(), e)
              })?;

    Ok(format!("{:#}", parsed))
}
