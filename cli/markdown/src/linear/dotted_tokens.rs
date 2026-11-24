//! A bare host name, such as `crates.io` or `jql.rs:167`, which Linear's
//! parser links when its last label is a top-level domain, or when it is the
//! numeric host of an email address.

use std::ops::Range;

use super::scheme_urls::SchemeUrl;
use super::text_runs::{RunCharacter, TextRun};
use super::{is_top_level_domain, Escape, Parsed};

pub(super) fn escapes(markdown: &str, events: &Parsed) -> Vec<Escape> {
    TextRun::from_events(markdown, events)
        .iter()
        .flat_map(|run| {
            linkable_tokens(run)
                .flat_map(DottedToken::unescaped_dots)
                .map(|before| Escape { before })
                .collect::<Vec<_>>()
        })
        .collect()
}

pub(super) fn linkable_tokens(
    run: &TextRun,
) -> impl Iterator<Item = DottedToken<'_>> {
    words(&run.characters).into_iter().flat_map(|word| {
        DottedToken::in_word(word.characters)
            .into_iter()
            .filter(move |token| token.is_linkable(&word))
    })
}

struct Word<'run> {
    characters: &'run [RunCharacter],
    follows_at_sign: bool,
}

fn words(characters: &[RunCharacter]) -> Vec<Word<'_>> {
    let mut in_url = vec![false; characters.len()];
    for span in SchemeUrl::spans(characters) {
        in_url[span].fill(true);
    }
    let mut words = Vec::new();
    let mut start = None;
    for (index, run) in characters.iter().enumerate() {
        let is_word_character = !in_url[index] && is_word(run.character);
        match (start, is_word_character) {
            (None, true) => start = Some(index),
            (Some(word_start), false) => {
                words.push(Word::of(characters, word_start..index));
                start = None;
            }
            _ => {}
        }
    }
    if let Some(word_start) = start {
        words.push(Word::of(characters, word_start..characters.len()));
    }
    words
}

impl<'run> Word<'run> {
    fn of(characters: &'run [RunCharacter], word: Range<usize>) -> Self {
        let follows_at_sign = word
            .start
            .checked_sub(1)
            .is_some_and(|preceding| characters[preceding].character == '@');
        Self {
            characters: &characters[word],
            follows_at_sign,
        }
    }
}

fn is_word(character: char) -> bool {
    is_label(character) || matches!(character, '.' | ':')
}

fn is_label(character: char) -> bool {
    character.is_alphanumeric() || matches!(character, '-' | '_')
}

#[derive(Debug)]
pub(super) struct DottedToken<'run> {
    characters: &'run [RunCharacter],
    host_length: usize,
    starts_word: bool,
}

impl<'run> DottedToken<'run> {
    /// Each `label ( "." label )+ ( ":" digits )?` in `word`, scanning from
    /// the left and resuming after anything that ends a token.
    pub(super) fn in_word(word: &'run [RunCharacter]) -> Vec<Self> {
        let at = |index: usize| word.get(index).map(|run| run.character);
        let label_end = |from: usize| {
            from + word[from..]
                .iter()
                .take_while(|run| is_label(run.character))
                .count()
        };
        let mut tokens = Vec::new();
        let mut index = 0;
        while index < word.len() {
            if !is_label(word[index].character) {
                index += 1;
                continue;
            }
            let start = index;
            index = label_end(index);
            let mut labels = 1;
            while at(index) == Some('.') && at(index + 1).is_some_and(is_label)
            {
                index = label_end(index + 1);
                labels += 1;
            }
            if labels < 2 {
                continue;
            }
            let host_end = index;
            if at(index) == Some(':')
                && at(index + 1).is_some_and(|next| next.is_ascii_digit())
            {
                index += 1 + word[index + 1..]
                    .iter()
                    .take_while(|run| run.character.is_ascii_digit())
                    .count();
            }
            tokens.push(Self {
                characters: &word[start..index],
                host_length: host_end - start,
                starts_word: start == 0,
            });
        }
        tokens
    }

    fn last_label(&self) -> String {
        let host = &self.characters[..self.host_length];
        let last_dot = host
            .iter()
            .rposition(|run| run.character == '.')
            .unwrap_or_default();
        host[last_dot + 1..]
            .iter()
            .map(|run| run.character)
            .collect()
    }

    fn is_linkable(&self, word: &Word<'_>) -> bool {
        let last_label = self.last_label();
        is_top_level_domain(&last_label)
            || !last_label.is_ascii()
            || (word.follows_at_sign && self.starts_word && self.is_numeric())
    }

    fn is_numeric(&self) -> bool {
        self.characters[..self.host_length]
            .iter()
            .all(|run| run.character == '.' || run.character.is_ascii_digit())
    }

    fn unescaped_dots(self) -> impl Iterator<Item = usize> + 'run {
        self.characters[..self.host_length]
            .iter()
            .filter(|run| run.character == '.' && !run.escaped)
            .map(|run| run.source)
    }

    pub(super) fn source(&self) -> Range<usize> {
        let first = self.characters.first().map_or(0, |run| run.source);
        let end = self
            .characters
            .last()
            .map_or(first, |run| run.source + run.character.len_utf8());
        first..end
    }
}
