use crate::{error::QueryCompilerError, lex::*};

/// Reflects choices which token has been seen recently.
pub enum TokenState {
    OpeningParenthese(usize),
    OpeningBrace(usize),
    ClosingBrace(usize),
    CombinedResultsKeyword(usize),
    Variable(usize),
}

impl TokenState {
    pub fn from_keyword(keyword: String, offset: usize) -> Option<Self> {
        match keyword.as_str()
        {
            KEYWORD_COMBINED_RESULT =>
            {
                Some(TokenState::CombinedResultsKeyword(offset))
            },
            _ => None,
        }
    }
}

/// Returns the position of a required character.
///
/// - `cursor` and `end` determine which substring of `statement` should be
///   scanned.
/// - `character`: the character whose position should be returned
/// - `keyword`: relevant for constructing the error message in case the
///   character has not been found. Primarily meant for constructing an error
///   message with semantics like "expected 'combined_result' (the keyword)
///   should have been closed with '}' (the character)".
///
/// The returned offset is absolute to the entire statement, not just
/// the scanned slice.
pub fn get_mandatory_succeeding_character_position(
    cursor: usize,
    end: usize,
    statement: &str,
    character: char,
    keyword: &'static str)
    -> Result<usize, QueryCompilerError> {
    Ok(cursor
        + statement[cursor..end]
            .find(character)
            .ok_or(
                QueryCompilerError::MissingCharacter(
                    character, keyword))?)
}
