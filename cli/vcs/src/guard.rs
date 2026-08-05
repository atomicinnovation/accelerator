//! Pure decision logic for `vcs guard`: which `git` subcommands have a jj
//! equivalent, and quote-aware compound-command splitting.
//!
//! Reproduces `hooks/vcs-guard.sh`'s blocklist, allowlist, and jj-equivalent
//! suggestions verbatim. The compound-command splitter is a declared
//! departure: it is quote-aware (tracking single- and double-quote state,
//! splitting on `&&`/`||`/`;`/`|` only when unquoted) rather than porting the
//! shell's quote-blind `sed`-then-split, which wrongly splits inside a quoted
//! argument such as `git commit -m "build && test"`.

/// The outcome of evaluating a Bash tool call's command text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GuardDecision {
    Allow,
    Block {
        subcommand: String,
        suggestion: String,
    },
}

const BLOCKED_SUBCOMMANDS: &[&str] = &[
    "status", "diff", "add", "commit", "log", "branch", "checkout", "switch",
    "merge", "rebase", "reset", "stash", "show",
];

/// Decides whether `command` contains a git VCS subcommand with a jj
/// equivalent.
///
/// `gh` and `rtk`-prefixed commands are allowed unconditionally, checked
/// against the whole command string before any splitting — matching the
/// shell's own whole-string prefix check, not a per-segment one, so
/// `gh pr view && git status` is allowed in its entirety.
///
/// Otherwise the command is split on unquoted `&&`/`||`/`;`/`|` and each
/// segment (in order) is checked against the blocklist; the first match
/// wins.
#[must_use]
pub fn decide(command: &str) -> GuardDecision {
    let trimmed = command.trim_start();
    if starts_with_command(trimmed, "gh") || starts_with_command(trimmed, "rtk")
    {
        return GuardDecision::Allow;
    }
    for segment in split_unquoted(command) {
        if let Some(subcommand) = matched_subcommand(segment.trim()) {
            return GuardDecision::Block {
                subcommand: subcommand.to_owned(),
                suggestion: suggestion_for(subcommand),
            };
        }
    }
    GuardDecision::Allow
}

fn starts_with_command(command: &str, word: &str) -> bool {
    command
        .strip_prefix(word)
        .is_some_and(|rest| rest.starts_with(char::is_whitespace))
}

fn matched_subcommand(segment: &str) -> Option<&'static str> {
    let mut tokens = segment.split_whitespace();
    if tokens.next()? != "git" {
        return None;
    }
    let candidate = tokens.next()?;
    BLOCKED_SUBCOMMANDS
        .iter()
        .copied()
        .find(|&subcommand| subcommand == candidate)
}

fn suggestion_for(subcommand: &str) -> String {
    match subcommand {
        "status" => "jj status".to_owned(),
        "diff" => "jj diff".to_owned(),
        "add" => {
            "(not needed — jj has no staging area; use jj commit directly)"
                .to_owned()
        }
        "commit" => "jj commit -m \"message\"".to_owned(),
        "log" => "jj log".to_owned(),
        "branch" => "jj bookmark list".to_owned(),
        "show" => "jj show".to_owned(),
        _ => "check jj documentation for equivalent".to_owned(),
    }
}

/// Splits `command` on unquoted `&&`, `||`, `;`, and `|`.
///
/// Single- and double-quoted regions are tracked independently: a quote
/// character of the other kind inside an open quote is literal, and a
/// backslash-escaped `"` inside a double-quoted region does not close it (a
/// backslash has no special meaning inside a single-quoted region). An
/// unterminated quote extends to the end of the string, so nothing after it
/// is treated as a separator.
fn split_unquoted(command: &str) -> Vec<String> {
    let characters: Vec<char> = command.chars().collect();
    let mut segments = Vec::new();
    let mut current = String::new();
    let mut in_single = false;
    let mut in_double = false;
    let mut index = 0;

    while index < characters.len() {
        let character = characters[index];

        if in_single {
            current.push(character);
            in_single = character != '\'';
            index += 1;
            continue;
        }

        if in_double {
            if character == '\\' && characters.get(index + 1) == Some(&'"') {
                current.push('\\');
                current.push('"');
                index += 2;
                continue;
            }
            current.push(character);
            in_double = character != '"';
            index += 1;
            continue;
        }

        match character {
            '\'' => {
                in_single = true;
                current.push(character);
                index += 1;
            }
            '"' => {
                in_double = true;
                current.push(character);
                index += 1;
            }
            '&' if characters.get(index + 1) == Some(&'&') => {
                segments.push(std::mem::take(&mut current));
                index += 2;
            }
            '|' if characters.get(index + 1) == Some(&'|') => {
                segments.push(std::mem::take(&mut current));
                index += 2;
            }
            ';' | '|' => {
                segments.push(std::mem::take(&mut current));
                index += 1;
            }
            other => {
                current.push(other);
                index += 1;
            }
        }
    }
    segments.push(current);
    segments
}

#[cfg(test)]
mod tests {
    use super::decide;
    use super::GuardDecision;

    fn allow() -> GuardDecision {
        GuardDecision::Allow
    }

    fn block(subcommand: &str, suggestion: &str) -> GuardDecision {
        GuardDecision::Block {
            subcommand: subcommand.to_owned(),
            suggestion: suggestion.to_owned(),
        }
    }

    #[test]
    fn every_blocked_subcommand_is_matched_with_its_suggestion() {
        let cases = [
            ("git status", block("status", "jj status")),
            ("git diff", block("diff", "jj diff")),
            (
                "git add file.txt",
                block(
                    "add",
                    "(not needed — jj has no staging area; use jj commit \
                     directly)",
                ),
            ),
            (
                "git commit -m \"message\"",
                block("commit", "jj commit -m \"message\""),
            ),
            ("git log", block("log", "jj log")),
            ("git branch", block("branch", "jj bookmark list")),
            (
                "git checkout main",
                block("checkout", "check jj documentation for equivalent"),
            ),
            (
                "git switch main",
                block("switch", "check jj documentation for equivalent"),
            ),
            (
                "git merge main",
                block("merge", "check jj documentation for equivalent"),
            ),
            (
                "git rebase main",
                block("rebase", "check jj documentation for equivalent"),
            ),
            (
                "git reset",
                block("reset", "check jj documentation for equivalent"),
            ),
            (
                "git stash",
                block("stash", "check jj documentation for equivalent"),
            ),
            ("git show", block("show", "jj show")),
        ];
        for (command, expected) in cases {
            assert_eq!(decide(command), expected, "command: {command}");
        }
    }

    #[test]
    fn git_commands_with_no_jj_equivalent_are_allowed() {
        let cases = [
            "git push",
            "git pull",
            "git fetch",
            "git remote -v",
            "git clone https://example.com/repo.git",
            "git config user.name",
            "git tag",
        ];
        for command in cases {
            assert_eq!(decide(command), allow(), "command: {command}");
        }
    }

    #[test]
    fn gh_and_rtk_are_allowed_unconditionally() {
        assert_eq!(decide("gh pr view"), allow());
        assert_eq!(decide("rtk git status"), allow());
    }

    #[test]
    fn gh_prefix_allows_the_whole_compound_command() {
        assert_eq!(decide("gh pr view && git status"), allow());
    }

    #[test]
    fn compound_commands_split_on_each_separator() {
        let cases = [
            ("git status && echo done", block("status", "jj status")),
            ("echo start && git status", block("status", "jj status")),
            ("echo start && echo done", allow()),
            ("git status || echo done", block("status", "jj status")),
            ("echo start || git status", block("status", "jj status")),
            ("echo start || echo done", allow()),
            ("git status ; echo done", block("status", "jj status")),
            ("echo start ; git status", block("status", "jj status")),
            ("echo start ; echo done", allow()),
            ("git status | echo done", block("status", "jj status")),
            ("echo start | git status", block("status", "jj status")),
            ("echo start | echo done", allow()),
        ];
        for (command, expected) in cases {
            assert_eq!(decide(command), expected, "command: {command}");
        }
    }

    #[test]
    fn a_quoted_separator_is_not_a_split_point() {
        assert_eq!(
            decide("git commit -m \"build && test\""),
            block("commit", "jj commit -m \"message\"")
        );
    }

    #[test]
    fn an_unterminated_quote_extends_to_the_end_of_the_string() {
        // No closing quote, so the trailing `&& git status` stays inside the
        // open quote and is never seen as a separate, blockable segment.
        assert_eq!(
            decide("git commit -m \"unterminated && git status"),
            block("commit", "jj commit -m \"message\"")
        );
    }

    #[test]
    fn an_escaped_quote_inside_a_quoted_segment_does_not_close_it() {
        assert_eq!(
            decide("git commit -m \"he said \\\"hi\\\" && bye\" && git status"),
            block("commit", "jj commit -m \"message\"")
        );
    }

    #[test]
    fn an_unescaped_double_quote_inside_single_quotes_is_literal() {
        assert_eq!(
            decide("echo 'contains \" quote' && git status"),
            block("status", "jj status")
        );
    }

    #[test]
    fn an_unescaped_single_quote_inside_double_quotes_is_literal() {
        assert_eq!(
            decide("echo \"contains ' quote\" && git status"),
            block("status", "jj status")
        );
    }

    #[test]
    fn a_separator_adjacent_to_a_quote_boundary_with_no_whitespace_still_splits(
    ) {
        assert_eq!(
            decide("echo \"a\"&&git status"),
            block("status", "jj status")
        );
    }
}
