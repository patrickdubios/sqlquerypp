mod combined_result;

pub use combined_result::{CombinedResultNode, CompleteCombinedResultNode};

pub trait Node {
    fn get_begin_position(&self) -> usize;
    fn get_scope_begin_position(&self) -> usize;
    fn get_end_position(&self) -> usize;
}
