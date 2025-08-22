use {
    pyo3::{exceptions::PyValueError, PyErr},
    sqlparser::parser::ParserError,
    thiserror::Error,
};

#[derive(Clone, Debug, Error)]
pub enum QueryCompilerError {
    #[error("expecting `{0}` after keyword `{1}`")]
    MissingCharacter(char, &'static str),

    #[error("nesting `{0}` within `{1}` is not supported")]
    UnsupportedNesting(&'static str, &'static str),

    #[error("directive `{0}` at offset `{1}` is incomplete")]
    DirectiveIncomplete(&'static str, usize),

    #[error("parsing inner query failed: {0}")]
    InnerQueryInvalid(String),

    #[error("resulting query is invalid: {0}, {1}")]
    ResultingQueryInvalid(String, ParserError),
}

impl From<QueryCompilerError> for PyErr {
    fn from(value: QueryCompilerError) -> Self {
        PyValueError::new_err(value.to_string())
    }
}
