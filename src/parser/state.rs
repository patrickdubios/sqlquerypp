use crate::{
    error::*,
    lex::*,
    parser::{
        nodes::{CombinedResultNode, CompleteCombinedResultNode},
        types::NodesState,
    },
    scanner::{get_mandatory_succeeding_character_position, TokenState},
};

/// Reflects the current parser state.
///
/// This makes heavy use of optionals and statefulness (i.e. is
/// heavily passed around as a mutable reference). It is quite
/// meant as a intermediate parser state automaton.
///
/// For the codegen phase, it's not recommended to use this low-level
/// intermediate state automaton. See `ParserState::finalize()`.
pub struct ParserState<'t> {
    statement: &'t String,
    seen_token_state: Option<TokenState>,
    combined_result_nodes_state: NodesState<CombinedResultNode>,
    offset: usize,
}

/// The final parser state. See `ParserState::finalize`.
pub struct FinalParserState {
    pub statement: String,
    pub combined_result_nodes: Vec<CompleteCombinedResultNode>,
}

impl<'t> ParserState<'t> {
    pub fn initialize(statement: &'t String) -> Self {
        Self { statement,
               seen_token_state: None,
               combined_result_nodes_state: NodesState::new(),
               offset: 0 }
    }

    ///
    /// Steps through the given statement, word by word, and internally
    /// updates the parser state accordingly (i.e. saves which parsed
    /// objects have been seen and which data they contain).
    pub fn parse(&mut self) -> Result<(), QueryCompilerError> {
        for word in self.statement.split(WORD_DELIMITER)
        {
            self.advance_word(word)?;
            self.advance_offset(word);
        }
        Ok(())
    }

    /// Transforms the intermediate state automaton into a `FinalParserState`.
    ///
    /// It's recommended to use this transformation because:
    /// - In contrast to the low-level state automaton it only makes use of
    ///   optionals where parsed objects' data model explicitly requires it.
    /// - It checks whether the parsed objects are as complete as the codegen
    ///   phase requires (i.e. whether the parsed SQL code was incomplete or
    ///   otherwise semantically invalid).
    pub fn finalize(&'t self) -> Result<FinalParserState, QueryCompilerError> {
        let final_state = FinalParserState { statement: self.statement
                                                            .clone(),
                                             combined_result_nodes:
                                                 self.get_complete_nodes()? };
        Ok(final_state)
    }

    fn get_complete_nodes(
        &self)
        -> Result<Vec<CompleteCombinedResultNode>, QueryCompilerError> {
        let converted =
            self.combined_result_nodes_state
                .all_nodes
                .iter()
                .map(|n| n.clone().try_into())
                .collect::<Vec<Result<CompleteCombinedResultNode,
                           QueryCompilerError>>>();

        let ok_variants =
            converted.iter()
                     .filter_map(|el| el.clone().ok())
                     .collect::<Vec<CompleteCombinedResultNode>>();

        let error_variants = converted.iter()
                                      .filter_map(|el| el.clone().err())
                                      .collect::<Vec<QueryCompilerError>>();

        if !error_variants.is_empty()
        {
            return Err(error_variants[0].clone());
        }

        Ok(ok_variants)
    }

    fn advance_word(&mut self, word: &str) -> Result<(), QueryCompilerError> {
        if let Some(next) = self.try_forward_to_keyword_seen_state(word)?
        {
            self.seen_token_state = Some(next);
            return Ok(());
        }

        if let Some(next) =
            self.try_forward_to_initiator_char_based_state(word)?
        {
            self.seen_token_state = Some(next);
            return Ok(());
        }

        Ok(())
    }

    fn advance_offset(&mut self, word: &str) {
        self.offset += word.len() + WORD_DELIMITER.len();
    }

    fn try_forward_to_keyword_seen_state(
        &mut self,
        word: &str)
        -> Result<Option<TokenState>, QueryCompilerError> {
        if VALID_KEYWORDS.contains(&word)
        {
            let current =
                TokenState::from_keyword(word.to_string(), self.offset)
                    .expect("checked per .contains() above");
            self.handle_transition(&current)?;
            return Ok(Some(current));
        }

        Ok(None)
    }

    fn try_forward_to_initiator_char_based_state(
        &mut self,
        word: &str)
        -> Result<Option<TokenState>, QueryCompilerError> {
        if let Some(initiator) = word.chars().nth(0)
        {
            let state_candidate =
                self.get_scanner_state_by_char_token(initiator);

            if let Some(state) = &state_candidate
            {
                self.handle_transition(state)?;
            }

            return Ok(state_candidate);
        }

        Ok(None)
    }

    fn get_scanner_state_by_char_token(&self,
                                       token: char)
                                       -> Option<TokenState> {
        match token
        {
            BRACE_START => Some(TokenState::OpeningBrace(self.offset)),
            BRACE_END => Some(TokenState::ClosingBrace(self.offset)),
            PARENTHESE_START =>
            {
                Some(TokenState::OpeningParenthese(self.offset))
            },
            VARIABLE_START => Some(TokenState::Variable(self.offset)),
            _ => None,
        }
    }

    fn handle_transition(&mut self,
                         current_token_state: &TokenState)
                         -> Result<(), QueryCompilerError> {
        let handles_combined_result_node =
            self.combined_result_nodes_state.current_node.is_some();

        match (&self.seen_token_state, current_token_state)
        {
            (_, TokenState::CombinedResultsKeyword(offset)) =>
            {
                self.handle_combined_results_keyword(offset)?
            },

            (_, TokenState::OpeningParenthese(offset))
                if handles_combined_result_node =>
            {
                self.attach_iteration_query(*offset + 1)?
            },

            (_, TokenState::Variable(offset))
                if handles_combined_result_node =>
            {
                self.attach_variable(*offset)?
            },

            (_, TokenState::OpeningBrace(offset))
                if handles_combined_result_node =>
            {
                self.mark_inner_query_begin(*offset)?
            },

            (_, TokenState::ClosingBrace(offset))
                if handles_combined_result_node =>
            {
                self.finalize_combined_result_node(offset)
            },

            _ =>
            {},
        }

        Ok(())
    }

    fn handle_combined_results_keyword(&mut self,
                                       offset: &usize)
                                       -> Result<(), QueryCompilerError> {
        if self.combined_result_nodes_state.current_node.is_some()
        {
            let err =
                QueryCompilerError::UnsupportedNesting(KEYWORD_COMBINED_RESULT,
                                                       KEYWORD_COMBINED_RESULT);
            return Err(err);
        }
        self.combined_result_nodes_state.current_node =
            Some(CombinedResultNode::new(*offset));
        Ok(())
    }

    fn attach_iteration_query(&mut self,
                              cursor: usize)
                              -> Result<(), QueryCompilerError> {
        if let Some(node) = &mut self.combined_result_nodes_state.current_node
        {
            let brace_start_pos = get_mandatory_succeeding_character_position(
                cursor,
                self.statement.len(),
                self.statement,
                BRACE_START,
                KEYWORD_COMBINED_RESULT,
            )?;

            let closing_brace_pos =
                get_mandatory_succeeding_character_position(
                    cursor,
                    brace_start_pos,
                    self.statement,
                    PARENTHESE_END,
                    KEYWORD_COMBINED_RESULT,
                )?;

            node.iteration_query =
                Some(self.statement[cursor .. closing_brace_pos].into());
        }
        Ok(())
    }

    fn attach_variable(&mut self,
                       cursor: usize)
                       -> Result<(), QueryCompilerError> {
        let words_beyond_cursor =
            self.statement[cursor ..].split(WORD_DELIMITER);
        if let Some(found_variable) = words_beyond_cursor.into_iter().nth(0)
        {
            if let Some(node) =
                &mut self.combined_result_nodes_state.current_node
            {
                node.iteration_item_variable =
                    Some(found_variable.trim().into());
            }
        }
        Ok(())
    }

    fn mark_inner_query_begin(&mut self,
                              cursor: usize)
                              -> Result<(), QueryCompilerError> {
        if let Some(node) = &mut self.combined_result_nodes_state.current_node
        {
            node.inner_query_begin = Some(cursor);
        }
        Ok(())
    }

    fn finalize_combined_result_node(&mut self, offset: &usize) {
        if let Some(node) = &mut self.combined_result_nodes_state.current_node
        {
            if let Some(begin) = node.inner_query_begin
            {
                let slice_start = begin + 1;
                let slice_end = *offset - 1;
                let slice = &self.statement[slice_start .. slice_end];
                node.inner_query = Some(slice.trim().into());
            }
            node.end_position = Some(*offset);
            self.combined_result_nodes_state
                .all_nodes
                .push(node.clone());
            self.combined_result_nodes_state.current_node = None;
        }
    }
}
