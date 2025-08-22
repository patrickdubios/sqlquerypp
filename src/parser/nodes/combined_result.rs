use {
    crate::{
        codegen::mysql84::MySql84NodeCompiler,
        error::QueryCompilerError,
        lex::KEYWORD_COMBINED_RESULT,
        parser::nodes::Node,
    },
    sqlparser::{
        ast::{helpers::attached_token::AttachedToken, *},
        dialect::GenericDialect,
        parser::Parser,
    },
};

#[derive(Clone, Debug)]
pub struct CombinedResultNode {
    pub begin_position: usize,
    pub end_position: Option<usize>,
    pub iteration_query: Option<String>,
    pub iteration_item_variable: Option<String>,
    pub inner_query_begin: Option<usize>,
    pub inner_query: Option<String>,
}

#[derive(Clone, Debug)]
pub struct CompleteCombinedResultNode {
    begin_position: usize,
    end_position: usize,
    iteration_query: String,
    iteration_item_variable: String,
    inner_query_begin: usize,
    inner_query: String,
}

impl CompleteCombinedResultNode {
    pub fn new(begin_position: usize,
               end_position: usize,
               iteration_query: String,
               iteration_item_variable: String,
               inner_query_begin: usize,
               inner_query: String)
               -> Self {
        Self { begin_position,
               end_position,
               iteration_query,
               iteration_item_variable,
               inner_query_begin,
               inner_query }
    }
}

impl Node for CompleteCombinedResultNode {
    fn get_begin_position(&self) -> usize {
        self.begin_position
    }

    fn get_scope_begin_position(&self) -> usize {
        self.inner_query_begin
    }

    fn get_end_position(&self) -> usize {
        self.end_position
    }
}

fn normalize_query(query: &str) -> String {
    query.replace("\n", " ")
         .replace("\r", " ")
         .split(' ')
         .filter(|&el| !el.is_empty())
         .collect::<Vec<&str>>()
         .join(" ")
}

impl PartialEq for CompleteCombinedResultNode {
    fn eq(&self, other: &Self) -> bool {
        self.begin_position == other.begin_position
        && self.end_position == other.end_position
        && self.iteration_query == other.iteration_query
        && self.iteration_item_variable == other.iteration_item_variable
        && self.inner_query_begin == other.inner_query_begin
        && normalize_query(&self.inner_query)
           == normalize_query(&other.inner_query)
    }
}

impl CombinedResultNode {
    pub fn new(begin_position: usize) -> Self {
        Self { begin_position,
               end_position: None,
               iteration_query: None,
               iteration_item_variable: None,
               inner_query_begin: None,
               inner_query: None }
    }
}

impl TryFrom<CombinedResultNode> for CompleteCombinedResultNode {
    type Error = QueryCompilerError;

    fn try_from(value: CombinedResultNode) -> Result<Self, Self::Error> {
        if value.iteration_query.is_none()
           || value.end_position.is_none()
           || value.iteration_item_variable.is_none()
           || value.inner_query_begin.is_none()
           || value.inner_query.is_none()
        {
            let err = QueryCompilerError::DirectiveIncomplete(
                KEYWORD_COMBINED_RESULT,
                value.begin_position);
            return Err(err);
        }

        let node =
            CompleteCombinedResultNode::new(value.begin_position,
                                            value.end_position.unwrap(),
                                            value.iteration_query.unwrap(),
                                            value.iteration_item_variable
                                                 .unwrap(),
                                            value.inner_query_begin.unwrap(),
                                            value.inner_query.unwrap());
        Ok(node)
    }
}

impl From<sqlparser::parser::ParserError> for QueryCompilerError {
    fn from(value: sqlparser::parser::ParserError) -> Self {
        Self::InnerQueryInvalid(value.to_string())
    }
}

impl MySql84NodeCompiler for CompleteCombinedResultNode {
    fn generate_code(&self) -> Result<String, QueryCompilerError> {
        let original_select =
            prepare_parser_with_query(&self.inner_query)?.parse_select()?;

        let final_select = compile_final_select(&original_select, self)?;

        Ok(final_select.to_string())
    }
}

fn compile_final_select(original_select: &Select,
                        node: &CompleteCombinedResultNode)
                        -> Result<Box<Query>, QueryCompilerError> {
    let original_select_column_idents =
        derive_original_select_columns(original_select);

    let cte_columns =
        construct_recursive_cte_columns(&original_select_column_idents);
    let cte_statement =
        construct_recursive_cte_statement(original_select, cte_columns, node)?;

    let select = compile_recursive_cte(original_select, cte_statement)?;
    Ok(select)
}

fn compile_recursive_cte(original_select: &Select,
                         cte_statement: With)
                         -> Result<Box<Query>, QueryCompilerError> {
    let original_select_column_idents =
        derive_original_select_columns(original_select);

    let where_fragments =
        derive_joined_table_column_names(original_select)
        .unwrap_or_default()
        .iter()
        .map(|identifier| format!("{identifier} IS NOT NULL"))
        .collect::<Vec<String>>()
        .join(" AND ");

    let mut select = prepare_parser_with_query(format!(
        "SELECT * FROM all_entries WHERE {where_fragments}"
    ).as_str())?.parse_query()?;
    select.with = Some(cte_statement);
    let mut select_body = select.body
                                .as_select()
                                .expect("our own query is a valid SELECT")
                                .clone();
    select_body.projection = original_select_column_idents
        .into_iter()
        .map(|ident| SelectItem::UnnamedExpr(Expr::Identifier(ident)))
        .collect();
    select.body = Box::new(SetExpr::Select(Box::new(select_body)));
    Ok(select)
}

fn derive_joined_table_column_names(original_select: &Select)
                                    -> Option<Vec<String>> {
    let original_select_column_ident_pairs =
        derive_fully_qualified_original_select_columns(original_select);

    if let TableFactor::Table { name,
                                alias,
                                .. } = &original_select.from[0].relation
    {
        let target_to_elide = if alias.is_some()
        {
            alias.clone().unwrap().to_string()
        }
        else
        {
            name.to_string()
        };

        return Some(original_select_column_ident_pairs.iter()
                                                      .filter(|&pair| {
                                                          pair.0.value
                                                          != target_to_elide
                                                      })
                                                      .map(|pair| {
                                                          pair.1.value.clone()
                                                      })
                                                      .collect());
    }

    None
}

fn construct_cte_with_iteration(node: &CompleteCombinedResultNode)
                                -> Result<With, QueryCompilerError> {
    let cte =
        Cte { alias: TableAlias { name: "loop_values".into(),
                                  columns: vec![] },
              query:
                  prepare_parser_with_query(&node.iteration_query)?
                  .parse_query()?,
              from: None,
              materialized: None,
              closing_paren_token: AttachedToken::empty() };
    let stmt = With { cte_tables: vec![cte],
                      recursive: false,
                      with_token: AttachedToken::empty() };
    Ok(stmt)
}

fn construct_recursive_cte_statement(original_select: &Select,
                                     cte_columns: Vec<TableAliasColumnDef>,
                                     node: &CompleteCombinedResultNode)
                                     -> Result<With, QueryCompilerError> {
    let cte =
        Cte { alias: TableAlias { name: "all_entries".into(),
                                  columns: cte_columns },
              query:
                  Box::new(construct_recursive_cte_query(original_select,
                                                         node)?),
              from: None,
              materialized: None,
              closing_paren_token: AttachedToken::empty() };
    let stmt = With { cte_tables: vec![cte],
                      recursive: true,
                      with_token: AttachedToken::empty() };
    Ok(stmt)
}

fn construct_recursive_cte_query(original_select: &Select,
                                 node: &CompleteCombinedResultNode)
                                 -> Result<Query, QueryCompilerError> {
    Ok(Query {
        body: Box::new(
            SetExpr::SetOperation {
                op: SetOperator::Union,
                set_quantifier: SetQuantifier::All,
                left: compile_cte_anchor(original_select, node)?,
                right: compile_cte_loop(original_select, node)?,
            }
        ),
        with: Some(construct_cte_with_iteration(node)?),
        order_by: None,
        limit_clause: None,
        fetch: None,
        locks: vec![],
        for_clause: None,
        settings: None,
        format_clause: None,
        pipe_operators: vec![],
    })
}

fn construct_recursive_cte_columns(original_select_column_idents: &[Ident])
                                   -> Vec<TableAliasColumnDef> {
    let mut cte_idents: Vec<Ident> = original_select_column_idents.into();
    cte_idents.insert(0, Ident::new("n"));

    cte_idents.into_iter()
              .map(|ident| TableAliasColumnDef { name: ident,
                                                 data_type: None })
              .collect()
}

fn derive_original_select_columns(original_select: &Select) -> Vec<Ident> {
    original_select.projection
                   .iter()
                   .filter_map(convert_select_item_to_ident_option)
                   .collect()
}

fn derive_fully_qualified_original_select_columns(original_select: &Select)
                                                  -> Vec<(Ident, Ident)> {
    original_select.projection
                   .iter()
                   .filter_map(
                        convert_select_item_to_full_qualified_idents_option
                   )
                   .collect()
}

fn convert_select_item_to_ident_option(item: &SelectItem) -> Option<Ident> {
    match item
    {
        SelectItem::UnnamedExpr(Expr::Identifier(ident)) => Some(ident.clone()),
        SelectItem::UnnamedExpr(Expr::CompoundIdentifier(idents)) =>
        {
            idents.last().cloned()
        },
        _ => None,
    }
}

fn convert_select_item_to_full_qualified_idents_option(
    item: &SelectItem)
    -> Option<(Ident, Ident)> {
    match item
    {
        SelectItem::UnnamedExpr(Expr::CompoundIdentifier(idents))
            if idents.len() == 2 =>
        {
            Some((idents[0].clone(), idents[1].clone()))
        },
        _ => None,
    }
}

fn compile_cte_anchor(original_select: &Select,
                      node: &CompleteCombinedResultNode)
                      -> Result<Box<SetExpr>, QueryCompilerError> {
    let mut cte_anchor = original_select.clone();
    if cte_anchor.from.len() != 1
    {
        let msg = "inner query may only have one table following
                   FROM directive";
        return Err(QueryCompilerError::InnerQueryInvalid(msg.into()));
    }

    insert_anchor_iteration_index(&mut cte_anchor)?;
    transform_all_joins_to_left_joins(&mut cte_anchor);
    apply_anchor_iteration_variable(&mut cte_anchor, node)?;

    Ok(Box::new(SetExpr::Select(Box::new(cte_anchor))))
}

fn apply_anchor_iteration_variable(cte_anchor: &mut Select,
                                   node: &CompleteCombinedResultNode)
                                   -> Result<(), QueryCompilerError> {
    if let Some(selection) = &cte_anchor.selection
    {
        let transformed_selection_fragment =
            selection.to_string()
                     .replace(&node.iteration_item_variable,
                              "(SELECT * FROM loop_values LIMIT 1)");
        let transformed_selection =
            prepare_parser_with_query(&transformed_selection_fragment)?
            .parse_expr()?;
        cte_anchor.selection = Some(transformed_selection);
    }
    Ok(())
}

fn transform_all_joins_to_left_joins(select: &mut Select) {
    select
        .from[0]
        .joins
        .iter_mut()
        .for_each(|join| {
        if let Some(constraint) = derive_join_constraint(join) {
            join.join_operator = JoinOperator::Left(constraint.clone());
        }
        });
}

fn insert_anchor_iteration_index(cte_anchor: &mut Select)
                                 -> Result<(), QueryCompilerError> {
    cte_anchor.projection.insert(
        0,
        SelectItem::UnnamedExpr(
            prepare_parser_with_query("0")?.parse_expr()?,
        )
    );
    Ok(())
}

fn derive_join_constraint(join: &Join) -> Option<&JoinConstraint> {
    match &join.join_operator
    {
        JoinOperator::Left(constraint) => Some(constraint),
        JoinOperator::Join(constraint) => Some(constraint),
        JoinOperator::Inner(constraint) => Some(constraint),
        JoinOperator::LeftOuter(constraint) => Some(constraint),
        JoinOperator::Right(constraint) => Some(constraint),
        JoinOperator::RightOuter(constraint) => Some(constraint),
        JoinOperator::FullOuter(constraint) => Some(constraint),
        JoinOperator::CrossJoin => None,
        JoinOperator::Semi(constraint) => Some(constraint),
        JoinOperator::LeftSemi(constraint) => Some(constraint),
        JoinOperator::RightSemi(constraint) => Some(constraint),
        JoinOperator::Anti(constraint) => Some(constraint),
        JoinOperator::LeftAnti(constraint) => Some(constraint),
        JoinOperator::RightAnti(constraint) => Some(constraint),
        JoinOperator::CrossApply => None,
        JoinOperator::OuterApply => None,
        JoinOperator::AsOf { match_condition: _,
                             constraint, } => Some(constraint),
        JoinOperator::StraightJoin(constraint) => Some(constraint),
    }
}

fn compile_cte_loop(original_select: &Select,
                    node: &CompleteCombinedResultNode)
                    -> Result<Box<SetExpr>, QueryCompilerError> {
    let mut cte_loop = original_select.clone();
    if cte_loop.from.len() != 1
    {
        let msg = "inner query may only have one table following
                   FROM directive";
        return Err(QueryCompilerError::InnerQueryInvalid(msg.into()));
    }
    insert_loop_iteration_index(&mut cte_loop)?;
    transform_loop_table_name_to_cte_alias(&mut cte_loop);
    transform_all_joins_to_left_joins(&mut cte_loop);
    add_loop_join(&mut cte_loop, node)?;
    finalize_selection(&mut cte_loop)?;

    Ok(Box::new(SetExpr::Select(Box::new(cte_loop))))
}

fn finalize_selection(cte_loop: &mut Select) -> Result<(), QueryCompilerError> {
    let lhs = prepare_parser_with_query("n + 1")?.parse_expr()?;

    let subquery =
        prepare_parser_with_query("SELECT COUNT(*) FROM loop_values")?
        .parse_query()?;

    cte_loop.selection =
        Some(Expr::BinaryOp { left: Box::new(lhs),
                              op: BinaryOperator::Lt,
                              right: Box::new(Expr::Subquery(subquery)) });

    Ok(())
}

fn add_loop_join(cte_loop: &mut Select,
                 node: &CompleteCombinedResultNode)
                 -> Result<(), QueryCompilerError> {
    let (loop_target_table_or_alias, loop_target_column) =
        extract_table_and_column_for_iteration_variable(cte_loop,
                                          &node.iteration_item_variable)?;
    let loop_target_column_name = loop_target_column.clone().value;

    let inner_select_table_name = extract_iteration_query_table_name(node)?;

    let constraint =
        construct_loop_join_constraint(&loop_target_table_or_alias,
                                       loop_target_column,
                                       loop_target_column_name)?;

    let table_factor =
        construct_loop_join_table_factor(loop_target_table_or_alias,
                                         inner_select_table_name)?;

    let join = Join { join_operator: JoinOperator::Left(constraint),
                      relation: table_factor,
                      global: false };
    cte_loop.from[0].joins.insert(0, join);
    Ok(())
}

fn construct_loop_join_table_factor(
    loop_target_table_or_alias: Ident,
    inner_select_table_name: String)
    -> Result<TableFactor, QueryCompilerError> {
    let join_alias = loop_target_table_or_alias.value;
    let table_factor =
        prepare_parser_with_query(
            format!("{inner_select_table_name} AS {join_alias}")
            .as_str())?
        .parse_table_factor()?;
    Ok(table_factor)
}

fn construct_loop_join_constraint(
    loop_target_table_or_alias: &Ident,
    loop_target_column: Ident,
    loop_target_column_name: String)
    -> Result<JoinConstraint, QueryCompilerError> {
    let join_subquery =
        prepare_parser_with_query(
            format!("SELECT {loop_target_column_name} FROM loop_values
                     WHERE {loop_target_column_name}
                           > all_entries.{loop_target_column_name}
                     LIMIT 1").as_str()
        )?.parse_query()?;
    let constraint =
        JoinConstraint::On(
            Expr::BinaryOp {
                left: Box::new(Expr::CompoundIdentifier(vec![
                    loop_target_table_or_alias.clone(),
                    loop_target_column,
                ])),
                op: BinaryOperator::Eq,
                right: Box::new(Expr::Subquery(join_subquery)),
            }
        );
    Ok(constraint)
}

fn extract_iteration_query_table_name(node: &CompleteCombinedResultNode)
                                      -> Result<String, QueryCompilerError> {
    let inner_select =
        prepare_parser_with_query(&node.inner_query)?.parse_select()?;
    if inner_select.from.len() != 1
    {
        return Err(QueryCompilerError::InnerQueryInvalid(
            "expected loop iteration query to have just one table
             followed by FROM".into()));
    }
    let inner_select_table_name = match &inner_select.from[0].relation {
        TableFactor::Table { name, ..} => Some(name.to_string()),
        _ => None,
    }.ok_or(QueryCompilerError::InnerQueryInvalid(
        "could not derive table name from loop iteration query".into()))?;
    Ok(inner_select_table_name)
}

fn extract_table_and_column_for_iteration_variable(
    cte_loop: &mut Select,
    iteration_variable: &str)
    -> Result<(Ident, Ident), QueryCompilerError> {
    let idents =
        extract_iteration_variable_idents(cte_loop, iteration_variable)?;
    if idents.len() != 2
    {
        return Err(QueryCompilerError::InnerQueryInvalid(
            "expected lvalue of iteration variable to be
             of schema: table.column".into()));
    }
    Ok((idents[0].clone(), idents[1].clone()))
}

fn extract_iteration_variable_idents(
    cte_loop: &mut Select,
    iteration_variable: &str)
    -> Result<Vec<Ident>, QueryCompilerError> {
    let err_candidate = QueryCompilerError::InnerQueryInvalid(
        "should contain iteration variable".into());
    let selection = cte_loop.selection.clone().ok_or(err_candidate.clone())?;
    let candidate = match &selection
                    {
                        Expr::BinaryOp { left: _,
                                         op: _,
                                         right: _, } =>
                        {
                            get_iteration_target_identifier(&selection,
                                                            iteration_variable)
                        },
                        _ => None,
                    }.ok_or(err_candidate.clone())?;
    let idents = match candidate
                 {
                     Expr::CompoundIdentifier(idents) => Some(idents),
                     _ => None,
                 }.ok_or(err_candidate)?;
    Ok(idents)
}

fn get_iteration_target_identifier(expr: &Expr,
                                   iteration_variable: &str)
                                   -> Option<Expr> {
    match expr
    {
        Expr::BinaryOp { left,
                         op,
                         right, } =>
        {
            if *op == BinaryOperator::Eq
            {
                if let Expr::Value(value_with_span) = &**right
                {
                    if let Value::Placeholder(var) = &value_with_span.value
                    {
                        if var == iteration_variable
                        {
                            return Some(*left.clone());
                        }
                    }
                }
            }
            get_iteration_target_identifier(expr, iteration_variable)
        },
        _ => None,
    }
}

fn transform_loop_table_name_to_cte_alias(cte_loop: &mut Select) {
    cte_loop.from[0].relation =
        TableFactor::Table { name: vec!["all_entries".into()].into(),
                             alias: None,
                             args: None,
                             with_hints: vec![],
                             version: None,
                             with_ordinality: false,
                             partitions: vec![],
                             json_path: None,
                             sample: None,
                             index_hints: vec![] };
}

fn insert_loop_iteration_index(cte_loop: &mut Select)
                               -> Result<(), QueryCompilerError> {
    cte_loop.projection.insert(
        0,
        SelectItem::UnnamedExpr(
            prepare_parser_with_query("n + 1")?.parse_expr()?,
        )
    );
    Ok(())
}

fn prepare_parser_with_query(query: &str)
                             -> Result<Parser<'_>, QueryCompilerError> {
    let parser = sqlparser::parser::Parser::new(&GenericDialect {});
    Ok(parser.try_with_sql(query)?)
}
