use {
    crate::{
        parser::ParserState,
        types::{CombinedResultNodeSlice, CompiledQueryDescriptor},
    },
    pyo3::prelude::*,
};

mod codegen;
mod error;
mod lex;
mod parser;
mod scanner;
mod types;

///
/// make_compiler_impl
///
/// This is a shorthand for generating a high-level compiler function.
macro_rules! make_compiler_impl {
    ($func_name:ident, $trait:ty) => {
        #[pyfunction]
        fn $func_name(statement: String) -> PyResult<CompiledQueryDescriptor> {
            use $trait;

            // First, we construct the parser. See ParserState.
            let mut parser = ParserState::initialize(&statement);

            // After that, we do all the lexical checks and parsing systematics.
            // The parser now contains a
            parser.parse()?;

            // When parsing, the parser deals with "intermediate structs" which
            // means, those intermediates heavily make use of "std::Option".
            // For the final code generation, it is not much helpful to always
            // have to check whether the parsed objects are complete.
            // This is what the separate state and the separate `Complete...`
            // datastructs are for. See `FinalParserState`.
            let mut finalized_state = parser.finalize()?;

            Ok(finalized_state.generate_code()?)
        }
    };
}

make_compiler_impl!(preprocess_mysql84_query, codegen::MySql84QueryCompiler);

/// Constructs the (internal!) sqlquerypp module containing helper
/// datastructs and compiler implementations.
#[pymodule]
fn sqlquerypp(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(preprocess_mysql84_query, m)?)?;

    m.add_class::<CompiledQueryDescriptor>()?;
    m.add_class::<CombinedResultNodeSlice>()?;

    Ok(())
}
