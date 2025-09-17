use {
    crate::{
        codegen::common::format_query_prettily,
        error::QueryCompilerError,
        parser::{FinalParserState, Node},
        types::{CombinedResultNodeSlice, CompiledQueryDescriptor},
    },
    std::cmp::Ordering,
};

/// A trait supposed to be implemented upon `FinalParserState`.
pub trait MySql84QueryCompiler {
    fn generate_code(&mut self)
                     -> Result<CompiledQueryDescriptor, QueryCompilerError>;
}

/// A trait supposed to be implemented upon any parsed node.
pub trait MySql84NodeCompiler {
    fn generate_code(&self) -> Result<String, QueryCompilerError>;
}

fn get_node_ordering_key(lhs: &impl Node, rhs: &impl Node) -> Ordering {
    if lhs.get_end_position() > rhs.get_end_position()
    {
        Ordering::Less
    }
    else
    {
        Ordering::Greater
    }
}

fn process_nodes_in_order(state: &mut FinalParserState)
                          -> Result<(), QueryCompilerError> {
    let mut nodes_in_order = get_all_nodes(state);
    nodes_in_order.sort_by(get_node_ordering_key);
    for node in nodes_in_order.iter()
    {
        let original = &state.statement
            [node.get_begin_position() .. node.get_end_position() + 1];
        let generated_code = node.generate_code()?;
        let replaced =
            state.statement
                 .replace(original, format!("({generated_code:#})").as_str());
        state.statement = replaced;
    }

    Ok(())
}

fn get_all_nodes(state: &mut FinalParserState)
                 -> Vec<impl Node + MySql84NodeCompiler> {
    // NOTE it should be sufficient to just extend this function in
    // case further nodes are being introduced. the remaining code
    // should be sufficiently generic
    state.combined_result_nodes.clone()
}

impl MySql84QueryCompiler for FinalParserState {
    fn generate_code(&mut self)
                     -> Result<CompiledQueryDescriptor, QueryCompilerError>
    {
        process_nodes_in_order(self)?;

        let combined_result_node_slices = self.combined_result_nodes
                                              .iter()
                                              .map(|node| {
                                                  CombinedResultNodeSlice {
                        scope_begin: node.get_scope_begin_position(),
                        scope_end: node.get_end_position(),
                    }
                                              })
                                              .collect();
        let descriptor =
            CompiledQueryDescriptor { statement:
                                          format_query_prettily(self.statement
                                                                    .as_str())?,
                                      combined_result_node_slices };

        Ok(descriptor)
    }
}
