pub struct NodesState<TNode> {
    pub all_nodes: Vec<TNode>,
    pub current_node: Option<TNode>,
}

impl<TNode> NodesState<TNode> {
    pub fn new() -> Self {
        Self { all_nodes: vec![],
               current_node: None }
    }
}
