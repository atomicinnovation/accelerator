//! Confining the research subagents. The researcher may run only the
//! research fetch, and write only findings and level notes; the composer may
//! run nothing, and write only findings.
//!
//! The command rule follows Claude Code's own matching of a `Bash(… *)`
//! allow rule, as measured release by release, and is stricter for the
//! constructs in [`STRICTER_THAN_MEASURED`].

use std::fmt;
use std::fmt::Display;
use std::fmt::Formatter;

pub const PERMITTED_PREFIX: &str = "accelerator research fetch ";

pub const DEFAULT_RESEARCHER: &str = "accelerator:researcher";

pub const DEFAULT_COMPOSER: &str = "accelerator:composer";

/// Only what the shell itself discards around a command. A wider trim would
/// drop a trailing carriage return the shell keeps as part of a word, such
/// as a redirection's target.
const SHELL_BLANKS: [char; 3] = [' ', '\t', '\n'];

/// Constructs Claude Code accepts that the guard blocks.
///
/// Together they let a researcher read an environment secret back through a
/// usage error's echo, or send it to a remote host through bash's
/// `/dev/tcp`. The fetch needs neither.
pub const STRICTER_THAN_MEASURED: [Construct; 2] =
    [Construct::Expansion, Construct::InputRedirection];

/// Who made a tool call, as the hook input reports it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolCall {
    agent_id: Option<String>,
    agent_type: Option<String>,
}

impl ToolCall {
    pub const fn new(
        agent_id: Option<String>,
        agent_type: Option<String>,
    ) -> Self {
        Self {
            agent_id,
            agent_type,
        }
    }

    pub fn is_subagent(&self) -> bool {
        self.agent_id.as_deref().is_some_and(|id| !id.is_empty())
    }

    pub fn agent_type(&self) -> &str {
        self.agent_type.as_deref().unwrap_or_default()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Researcher,
    Composer,
}

impl Role {
    pub const fn may_write(self, file: TopicFile) -> bool {
        match (self, file) {
            (Self::Researcher, TopicFile::Finding | TopicFile::LevelNote)
            | (Self::Composer, TopicFile::Finding) => true,
            (Self::Composer, TopicFile::LevelNote) => false,
        }
    }

    const fn default_name(self) -> &'static str {
        match self {
            Self::Researcher => DEFAULT_RESEARCHER,
            Self::Composer => DEFAULT_COMPOSER,
        }
    }

    const fn agent_key(self) -> &'static str {
        match self {
            Self::Researcher => "agents.researcher",
            Self::Composer => "agents.composer",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfinedAgent {
    role: Role,
    configured_as: Option<String>,
}

impl ConfinedAgent {
    pub const fn role(&self) -> Role {
        self.role
    }
}

impl Display for ConfinedAgent {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match &self.configured_as {
            None => formatter.write_str(self.role.default_name()),
            Some(name) => {
                write!(formatter, "{name} ({})", self.role.agent_key())
            }
        }
    }
}

/// The agent names confined to each role: always the plugin's own, and any
/// name `agents.researcher` or `agents.composer` configures.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Confinement {
    researcher: Option<String>,
    composer: Option<String>,
}

impl Confinement {
    #[must_use]
    pub fn with_researcher(self, name: &str) -> Self {
        Self {
            researcher: Some(name.to_owned()),
            ..self
        }
    }

    #[must_use]
    pub fn with_composer(self, name: &str) -> Self {
        Self {
            composer: Some(name.to_owned()),
            ..self
        }
    }

    /// A name configured for both roles is confined as the researcher, the
    /// wider scope a misconfiguration most plausibly intends.
    pub fn identify(&self, agent_type: &str) -> Option<ConfinedAgent> {
        let by_default = [Role::Researcher, Role::Composer]
            .into_iter()
            .find(|role| role.default_name() == agent_type)
            .map(|role| ConfinedAgent {
                role,
                configured_as: None,
            });
        by_default.or_else(|| {
            [
                (Role::Researcher, &self.researcher),
                (Role::Composer, &self.composer),
            ]
            .into_iter()
            .find(|(_, name)| name.as_deref() == Some(agent_type))
            .map(|(role, _)| ConfinedAgent {
                role,
                configured_as: Some(agent_type.to_owned()),
            })
        })
    }
}

/// The agent a call must be confined as, if any. The main thread and every
/// other subagent go unconfined.
pub fn confined_agent(
    call: &ToolCall,
    confinement: &Confinement,
) -> Option<ConfinedAgent> {
    if call.is_subagent() {
        confinement.identify(call.agent_type())
    } else {
        None
    }
}

/// A write target's path below the research topics directory.
///
/// Whoever constructs one vouches that the path was resolved against the
/// real filesystem and crosses no symlink below the topics directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TopicsRelativePath(String);

impl TopicsRelativePath {
    pub const fn new(path: String) -> Self {
        Self(path)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Why a write target could not be placed below the topics directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathRejection {
    NotUnderTopics,
    DotComponent,
    Symlink(String),
    Uninspectable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TopicFile {
    Finding,
    LevelNote,
}

pub trait TopicLayout {
    fn classify(&self, target: &TopicsRelativePath) -> Option<TopicFile>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    Command(String),
    Write(Result<TopicsRelativePath, PathRejection>),
    Unreadable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision {
    Pass,
    Block(Block),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Block {
    WrongCommand,
    NoCommands,
    Syntax(Construct),
    Write(WriteRefusal),
    Unreadable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WriteRefusal {
    OutsideScope(TopicsRelativePath),
    Rejected(PathRejection),
}

/// A shell construct that could run, redirect, or expand something beyond
/// the fetch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Construct {
    Separator,
    Pipe,
    Ampersand,
    Newline,
    LineContinuation,
    Expansion,
    Backtick,
    ProcessSubstitution,
    Parenthesis,
    Brace,
    InputRedirection,
    OutputRedirection,
}

impl Display for Construct {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Separator => "a ';' separator",
            Self::Pipe => "a '|' pipe",
            Self::Ampersand => "a '&' operator",
            Self::Newline => "a newline",
            Self::LineContinuation => "a backslash-newline",
            Self::Expansion => "a '$' expansion",
            Self::Backtick => "a '`' substitution",
            Self::ProcessSubstitution => "a '=(…)' substitution",
            Self::Parenthesis => "a parenthesis",
            Self::Brace => "a brace",
            Self::InputRedirection => "an input redirection",
            Self::OutputRedirection => {
                "an output redirection other than to /dev/null"
            }
        })
    }
}

pub fn decide(
    action: &Action,
    agent: &ConfinedAgent,
    layout: &dyn TopicLayout,
) -> Decision {
    match action {
        Action::Command(command) => match agent.role {
            Role::Researcher => {
                command_decision(command.trim_matches(SHELL_BLANKS))
            }
            Role::Composer => Decision::Block(Block::NoCommands),
        },
        Action::Write(Ok(target))
            if layout
                .classify(target)
                .is_some_and(|file| agent.role.may_write(file)) =>
        {
            Decision::Pass
        }
        Action::Write(Ok(target)) => Decision::Block(Block::Write(
            WriteRefusal::OutsideScope(target.clone()),
        )),
        Action::Write(Err(rejection)) => Decision::Block(Block::Write(
            WriteRefusal::Rejected(rejection.clone()),
        )),
        Action::Unreadable => Decision::Block(Block::Unreadable),
    }
}

pub fn command_decision(command: &str) -> Decision {
    let Some(arguments) = command.strip_prefix(PERMITTED_PREFIX) else {
        return Decision::Block(Block::WrongCommand);
    };
    Lexer::new(arguments)
        .first_construct()
        .map_or(Decision::Pass, |construct| {
            Decision::Block(Block::Syntax(construct))
        })
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Quoting {
    Unquoted,
    Single,
    Double,
    Comment,
}

/// Scans the fetch's arguments as bash and zsh would tokenise them, far
/// enough to find the first construct that blocks.
struct Lexer {
    chars: Vec<char>,
    at: usize,
    quoting: Quoting,
    at_word_start: bool,
}

impl Lexer {
    fn new(arguments: &str) -> Self {
        Self {
            chars: arguments.chars().collect(),
            at: 0,
            quoting: Quoting::Unquoted,
            at_word_start: true,
        }
    }

    fn peek(&self, offset: usize) -> Option<char> {
        self.chars.get(self.at + offset).copied()
    }

    /// The shell deletes a backslash-newline pair before tokenising, joining
    /// the text either side, so it is refused rather than modelled.
    fn continues_line(&self) -> bool {
        matches!(
            (self.peek(1), self.peek(2)),
            (Some('\n'), _) | (Some('\r'), Some('\n'))
        )
    }

    fn first_construct(mut self) -> Option<Construct> {
        while let Some(current) = self.peek(0) {
            let blocked = match self.quoting {
                Quoting::Unquoted => self.unquoted(current),
                Quoting::Single => {
                    if current == '\'' {
                        self.quoting = Quoting::Unquoted;
                    }
                    Ok(1)
                }
                Quoting::Double => self.double_quoted(current),
                Quoting::Comment => {
                    if current == '\n' {
                        Err(Construct::Newline)
                    } else {
                        Ok(1)
                    }
                }
            };
            match blocked {
                Ok(consumed) => self.at += consumed,
                Err(construct) => return Some(construct),
            }
        }
        None
    }

    fn unquoted(&mut self, current: char) -> Result<usize, Construct> {
        let starts_word = std::mem::replace(&mut self.at_word_start, false);
        match current {
            ' ' | '\t' => {
                self.at_word_start = true;
                Ok(1)
            }
            '\\' if self.continues_line() => Err(Construct::LineContinuation),
            '\\' => Ok(2),
            '\'' => {
                self.quoting = Quoting::Single;
                Ok(1)
            }
            '"' => {
                self.quoting = Quoting::Double;
                Ok(1)
            }
            '#' if starts_word => {
                self.quoting = Quoting::Comment;
                Ok(1)
            }
            '=' if self.peek(1) == Some('(') => {
                Err(Construct::ProcessSubstitution)
            }
            '>' => self.output_redirection(),
            '&' if self.peek(1) == Some('>') => {
                Err(Construct::OutputRedirection)
            }
            _ => Self::unquoted_construct(current).map_or(Ok(1), Err),
        }
    }

    const fn unquoted_construct(current: char) -> Option<Construct> {
        match current {
            '\n' => Some(Construct::Newline),
            '$' => Some(Construct::Expansion),
            '`' => Some(Construct::Backtick),
            ';' => Some(Construct::Separator),
            '|' => Some(Construct::Pipe),
            '&' => Some(Construct::Ampersand),
            '(' | ')' => Some(Construct::Parenthesis),
            '{' | '}' => Some(Construct::Brace),
            '<' => Some(Construct::InputRedirection),
            _ => None,
        }
    }

    fn double_quoted(&mut self, current: char) -> Result<usize, Construct> {
        match current {
            '"' => {
                self.quoting = Quoting::Unquoted;
                Ok(1)
            }
            '\\' if self.continues_line() => Err(Construct::LineContinuation),
            '\\' => Ok(2),
            '$' => Err(Construct::Expansion),
            '`' => Err(Construct::Backtick),
            _ => Ok(1),
        }
    }

    /// Admits only descriptor duplication, `>&N`, and discarding to
    /// `/dev/null` with `>` or `>>`, each ending the word.
    fn output_redirection(&self) -> Result<usize, Construct> {
        let shape = if self.peek(1) == Some('&') {
            self.duplication()
        } else {
            self.discard()
        };
        shape
            .filter(|&length| self.ends_word(length))
            .ok_or(Construct::OutputRedirection)
    }

    fn duplication(&self) -> Option<usize> {
        let digits = self.count_from(2, |next| next.is_ascii_digit());
        (digits > 0).then_some(2 + digits)
    }

    fn discard(&self) -> Option<usize> {
        let operator = if self.peek(1) == Some('>') { 2 } else { 1 };
        let blanks =
            self.count_from(operator, |next| matches!(next, ' ' | '\t'));
        let target = operator + blanks;
        "/dev/null"
            .chars()
            .enumerate()
            .all(|(index, expected)| {
                self.peek(target + index) == Some(expected)
            })
            .then_some(target + "/dev/null".len())
    }

    fn count_from(
        &self,
        offset: usize,
        admits: impl Fn(char) -> bool,
    ) -> usize {
        self.chars[(self.at + offset).min(self.chars.len())..]
            .iter()
            .take_while(|&&next| admits(next))
            .count()
    }

    fn ends_word(&self, length: usize) -> bool {
        matches!(self.peek(length), None | Some(' ' | '\t'))
    }
}

/// Why a confined call was blocked, as the confined agent reads it.
pub struct Refusal<'a> {
    pub agent: &'a ConfinedAgent,
    pub topics: &'a str,
    pub block: &'a Block,
}

impl Display for Refusal<'_> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        let agent = self.agent;
        match self.block {
            Block::WrongCommand => write!(
                formatter,
                "E_RESEARCH_GUARD_COMMAND: {agent} may run only \
                 '{PERMITTED_PREFIX}…'"
            ),
            Block::NoCommands => write!(
                formatter,
                "E_RESEARCH_GUARD_COMMAND: {agent} may run no commands"
            ),
            Block::Syntax(construct) => write!(
                formatter,
                "E_RESEARCH_GUARD_SYNTAX: command contains {construct} — \
                 pass the query as one single-quoted argument"
            ),
            Block::Write(refusal) => {
                write!(
                    formatter,
                    "E_RESEARCH_GUARD_WRITE: {agent} may write only \
                     {}/<set>/findings/<nn>-<name>.md",
                    self.topics
                )?;
                if agent.role.may_write(TopicFile::LevelNote) {
                    write!(
                        formatter,
                        " or {}/<set>/findings/<nn>-<name>.levels/\
                         <lineage>.md",
                        self.topics
                    )?;
                }
                formatter.write_str(" — ")?;
                self.write_cause(refusal, formatter)
            }
            Block::Unreadable => write!(
                formatter,
                "E_RESEARCH_GUARD_UNREADABLE: {agent} tool call has no \
                 readable command or path"
            ),
        }
    }
}

impl Refusal<'_> {
    fn write_cause(
        &self,
        refusal: &WriteRefusal,
        formatter: &mut Formatter<'_>,
    ) -> fmt::Result {
        match refusal {
            WriteRefusal::OutsideScope(target) => {
                write!(formatter, "'{}' is outside that scope", target.as_str())
            }
            WriteRefusal::Rejected(PathRejection::NotUnderTopics) => {
                write!(formatter, "not under {}", self.topics)
            }
            WriteRefusal::Rejected(PathRejection::DotComponent) => {
                formatter.write_str("a '.' or '..' component")
            }
            WriteRefusal::Rejected(PathRejection::Symlink(component)) => {
                write!(formatter, "a symlink at '{component}'")
            }
            WriteRefusal::Rejected(PathRejection::Uninspectable) => formatter
                .write_str("a path component that could not be inspected"),
        }
    }
}

/// The reason given when the guard itself fails while judging a confined
/// call, which must block rather than let the call through.
pub struct InternalFailure<'a>(pub &'a ConfinedAgent);

impl Display for InternalFailure<'_> {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "E_RESEARCH_GUARD_INTERNAL: {} tool call blocked — the guard \
             failed while judging it",
            self.0
        )
    }
}
