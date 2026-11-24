//! The URLs Linear links whole, wherever they sit in a text run.

use std::ops::Range;

use super::text_runs::RunCharacter;

const SCHEMES: [&str; 4] = ["http://", "https://", "ftp://", "mailto:"];

const SHED_FROM_THE_END: [char; 9] =
    ['.', ',', ':', ';', '!', '?', '"', '\'', '*'];

pub(super) struct SchemeUrl;

impl SchemeUrl {
    /// The index ranges of `characters` that Linear would link as URLs.
    pub(super) fn spans(characters: &[RunCharacter]) -> Vec<Range<usize>> {
        let mut spans = Vec::new();
        let mut index = 0;
        while index < characters.len() {
            if opens_url(characters, index) {
                let end = url_end(characters, index);
                spans.push(index..end);
                index = end.max(index + 1);
            } else {
                index += 1;
            }
        }
        spans
    }
}

fn opens_url(characters: &[RunCharacter], index: usize) -> bool {
    let preceding = index
        .checked_sub(1)
        .map(|preceding| characters[preceding].character);
    let rest = &characters[index..];
    let follows_a_letter_or_digit =
        preceding.is_some_and(|character| character.is_ascii_alphanumeric());
    let follows_whitespace_or_nothing =
        preceding.is_none_or(char::is_whitespace);
    (!follows_a_letter_or_digit
        && SCHEMES.iter().any(|scheme| starts_with(rest, scheme)))
        || (follows_whitespace_or_nothing && starts_with(rest, "//"))
}

fn starts_with(characters: &[RunCharacter], prefix: &str) -> bool {
    characters.len() >= prefix.chars().count()
        && characters
            .iter()
            .zip(prefix.chars())
            .all(|(run, expected)| {
                run.character.eq_ignore_ascii_case(&expected)
            })
}

fn url_end(characters: &[RunCharacter], start: usize) -> usize {
    let mut end = start
        + characters[start..]
            .iter()
            .take_while(|run| {
                !run.character.is_whitespace()
                    && !matches!(run.character, '<' | '>')
            })
            .count();
    while end > start {
        let last = characters[end - 1].character;
        if SHED_FROM_THE_END.contains(&last)
            || (last == ')' && closes_unopened(&characters[start..end]))
        {
            end -= 1;
        } else {
            break;
        }
    }
    end
}

fn closes_unopened(url: &[RunCharacter]) -> bool {
    let count =
        |bracket| url.iter().filter(|run| run.character == bracket).count();
    count(')') > count('(')
}
