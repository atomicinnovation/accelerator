//! The cosmetic rewrites Linear applies to a Markdown description it stores,
//! each as a pure function a recording tracker can apply.

/// One rewrite Linear was observed to make to a description it stores.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rewrite {
    /// `~550` → `\~550`.
    EscapeTildes,
    /// Bold wrapped across lines is closed and reopened on each line.
    BoldPerLine,
    /// Bold wrapping a word and a code span closes before the span.
    BoldOffCode,
    /// `[t](u)` → `[t](<u>)`.
    AngleBracketDestinations,
    /// A bare `https://…` becomes `[u](<u>)`.
    AutolinkUrls,
    /// A bare `name.tld` becomes `[name.tld](<http://name.tld>)`. Unlike
    /// the others, this changes meaning: the text becomes a link.
    AutolinkDottedTokens,
    /// `- a` → `* a`; task items keep `-`.
    StarBullets,
    /// `[x]` → `[X]`.
    UppercaseTaskMarkers,
    /// `|---|---|` → `| -- | -- |`.
    SpacedTableDelimiters,
    /// A continuation line opening `0160.` reads `0160\.`.
    EscapeWrappedNumbers,
    /// A line break inside a code span becomes a space, the continuation
    /// line losing its container's content indent.
    JoinCodeSpanLines,
    /// A list directly after a paragraph gains a blank line before it.
    BlankLineBeforeList,
}

impl Rewrite {
    #[must_use]
    pub fn apply(self, markdown: &str) -> String {
        match self {
            Self::EscapeTildes => escape_tildes(markdown),
            Self::BoldPerLine => bold_per_line(markdown),
            Self::BoldOffCode => by_line(markdown, bold_off_code),
            Self::AngleBracketDestinations => {
                by_line(markdown, angle_bracket_destinations)
            }
            Self::AutolinkUrls => by_line(markdown, autolink_urls),
            Self::AutolinkDottedTokens => {
                by_line(markdown, autolink_dotted_tokens)
            }
            Self::StarBullets => by_line(markdown, star_bullet),
            Self::UppercaseTaskMarkers => {
                by_line(markdown, |line| line.replace("[x]", "[X]"))
            }
            Self::SpacedTableDelimiters => by_line(markdown, spaced_delimiters),
            Self::EscapeWrappedNumbers => escape_wrapped_numbers(markdown),
            Self::JoinCodeSpanLines => join_code_span_lines(markdown),
            Self::BlankLineBeforeList => blank_line_before_list(markdown),
        }
    }
}

fn by_line(markdown: &str, rewrite: impl FnMut(&str) -> String) -> String {
    let mut lines: Vec<String> = markdown.split('\n').map(rewrite).collect();
    if markdown.ends_with('\n') {
        lines.pop();
        lines.push(String::new());
    }
    lines.join("\n")
}

fn escape_tildes(markdown: &str) -> String {
    let mut out = String::new();
    let mut in_code = false;
    let mut previous = None;
    for character in markdown.chars() {
        if character == '`' {
            in_code = !in_code;
        }
        if character == '~' && !in_code && previous != Some('\\') {
            out.push('\\');
        }
        out.push(character);
        previous = Some(character);
    }
    out
}

fn quote_prefix(line: &str) -> (&str, &str) {
    line.strip_prefix("> ")
        .map_or(("", line), |content| ("> ", content))
}

fn bold_per_line(markdown: &str) -> String {
    let mut open = false;
    by_line(markdown, |line| {
        let (prefix, content) = quote_prefix(line);
        let mut content = if open {
            format!("**{content}")
        } else {
            content.to_owned()
        };
        open = content.matches("**").count() % 2 == 1;
        if open {
            content.push_str("**");
        }
        format!("{prefix}{content}")
    })
}

fn bold_off_code(line: &str) -> String {
    let Some(close) = line.find("`**") else {
        return line.to_owned();
    };
    let Some(code_start) = line[..close].rfind('`') else {
        return line.to_owned();
    };
    let Some(bold_start) = line[..code_start].rfind("**") else {
        return line.to_owned();
    };
    let lead = &line[bold_start + 2..code_start];
    if !lead.ends_with(' ') {
        return line.to_owned();
    }
    format!(
        "{}**{}** {}{}",
        &line[..bold_start],
        lead.trim_end(),
        &line[code_start..=close],
        &line[close + 3..]
    )
}

fn angle_bracket_destinations(line: &str) -> String {
    let mut out = String::new();
    let mut rest = line;
    while let Some(start) = rest.find("](") {
        let (before, after) = rest.split_at(start + 2);
        out.push_str(before);
        if after.starts_with('<') {
            rest = after;
            continue;
        }
        let Some(end) = after.find(')') else {
            rest = after;
            break;
        };
        out.push('<');
        out.push_str(&after[..end]);
        out.push_str(">)");
        rest = &after[end + 1..];
    }
    out.push_str(rest);
    out
}

fn link_words(line: &str, as_link: impl Fn(&str) -> Option<String>) -> String {
    let mut out = String::new();
    let mut word = String::new();
    let flush = |word: &mut String, out: &mut String| {
        out.push_str(&as_link(word).unwrap_or_else(|| word.clone()));
        word.clear();
    };
    for character in line.chars() {
        if character.is_whitespace() {
            flush(&mut word, &mut out);
            out.push(character);
        } else {
            word.push(character);
        }
    }
    flush(&mut word, &mut out);
    out
}

fn autolink_urls(line: &str) -> String {
    link_words(line, |word| {
        (word.starts_with("https://") || word.starts_with("http://"))
            .then(|| format!("[{word}](<{word}>)"))
    })
}

fn autolink_dotted_tokens(line: &str) -> String {
    link_words(line, |word| {
        let (name, tld) = word.rsplit_once('.')?;
        let named = !name.is_empty()
            && name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
        let top_level =
            tld.len() >= 2 && tld.chars().all(|c| c.is_ascii_lowercase());
        (named && top_level).then(|| format!("[{word}](<http://{word}>)"))
    })
}

fn star_bullet(line: &str) -> String {
    let content = line.trim_start();
    let indent = &line[..line.len() - content.len()];
    match content.strip_prefix("- ") {
        Some(item) if !item.starts_with('[') => format!("{indent}* {item}"),
        _ => line.to_owned(),
    }
}

fn spaced_delimiters(line: &str) -> String {
    let cells: Vec<&str> = line.trim().trim_matches('|').split('|').collect();
    let is_delimiter = line.trim().starts_with('|')
        && cells
            .iter()
            .all(|cell| !cell.is_empty() && cell.chars().all(|c| c == '-'));
    if is_delimiter {
        format!("|{}|", vec![" -- "; cells.len()].join("|"))
    } else {
        line.to_owned()
    }
}

fn escape_wrapped_numbers(markdown: &str) -> String {
    let mut previous_blank = true;
    by_line(markdown, |line| {
        let content = line.trim_start();
        let digits = content.chars().take_while(char::is_ascii_digit).count();
        let continues =
            !previous_blank && digits > 0 && content[digits..].starts_with('.');
        previous_blank = line.trim().is_empty();
        if continues {
            let indent = line.len() - content.len();
            format!(
                "{}\\{}",
                &line[..indent + digits],
                &line[indent + digits..]
            )
        } else {
            line.to_owned()
        }
    })
}

fn is_list_item(line: &str) -> bool {
    let content = line.trim_start();
    content.starts_with("- ") || content.starts_with("* ")
}

fn join_code_span_lines(markdown: &str) -> String {
    let mut out: Vec<String> = Vec::new();
    let mut joining: Option<usize> = None;
    for line in markdown.split('\n') {
        match joining {
            Some(content_indent) => {
                let kept = line
                    .char_indices()
                    .take(content_indent)
                    .take_while(|(_, c)| *c == ' ')
                    .count();
                let last = out.last_mut().unwrap_or_else(|| unreachable!());
                last.push(' ');
                last.push_str(&line[kept..]);
            }
            None => out.push(line.to_owned()),
        }
        let last = out.last().unwrap_or_else(|| unreachable!());
        joining = (last.matches('`').count() % 2 == 1).then(|| {
            if is_list_item(last) {
                2
            } else {
                0
            }
        });
    }
    out.join("\n")
}

fn blank_line_before_list(markdown: &str) -> String {
    let mut out: Vec<&str> = Vec::new();
    for line in markdown.split('\n') {
        if let Some(previous) = out.last() {
            if is_list_item(line)
                && !previous.trim().is_empty()
                && !is_list_item(previous)
            {
                out.push("");
            }
        }
        out.push(line);
    }
    out.join("\n")
}
