//! The fair queue arXiv fetches wait in: tickets issued in FIFO order, each
//! bound to the arguments of the call it was issued for, kept across calls
//! until it expires or passes its cap.

use std::fmt;
use std::time::Duration;
use std::time::SystemTime;

use crate::sources::classify::Reason;
use crate::sources::request::ArxivId;
use crate::sources::request::ArxivQuery;
use crate::sources::request::ArxivRequest;
use crate::sources::request::Limit;
use crate::sources::request::RequestError;
use crate::sources::schedule::Deadline;

/// How long an absent ticket keeps its place after its last call ended.
pub const EXPIRY: Duration = Duration::from_secs(300);
/// How long after issue a ticket may still be re-presented.
pub const CAP: Duration = Duration::from_secs(900);
pub const LONGEST_INVOCATION: Duration = Duration::from_secs(100);
/// No healthy call holds a ticket longer: re-presented at the cap, then live
/// for one whole invocation.
pub const ABANDONED_AFTER: Duration = CAP.saturating_add(LONGEST_INVOCATION);

/// Six lowercase hex digits telling a ticket apart from another issued the
/// same number after its queue was wiped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Nonce(u32);

impl Nonce {
    const DIGITS: usize = 6;
    const BITS: u32 = 0x00ff_ffff;

    pub const fn from_bits(bits: u32) -> Self {
        Self(bits & Self::BITS)
    }

    /// # Errors
    ///
    /// [`RequestError::TicketMalformed`] for anything but six lowercase hex
    /// digits.
    pub fn from_hex(raw: &str) -> Result<Self, RequestError> {
        let well_formed = raw.len() == Self::DIGITS
            && raw.chars().all(|c| matches!(c, '0'..='9' | 'a'..='f'));
        well_formed
            .then(|| u32::from_str_radix(raw, 16).ok())
            .flatten()
            .map(Self)
            .ok_or_else(|| RequestError::TicketMalformed(raw.to_owned()))
    }
}

impl fmt::Display for Nonce {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{:06x}", self.0)
    }
}

/// A place in the queue, written `<number>-<nonce>`. Equal only when both
/// parts are; ordered in the queue by number alone.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Ticket {
    number: u64,
    nonce: Nonce,
}

impl Ticket {
    pub const fn issue(number: u64, nonce: Nonce) -> Self {
        Self { number, nonce }
    }

    /// # Errors
    ///
    /// [`RequestError::TicketMalformed`] for anything but the exact form a
    /// ticket is printed in.
    pub fn parse(raw: &str) -> Result<Self, RequestError> {
        let malformed = || RequestError::TicketMalformed(raw.to_owned());
        let (number, nonce) = raw.split_once('-').ok_or_else(malformed)?;
        if number.is_empty() || !number.chars().all(|c| c.is_ascii_digit()) {
            return Err(malformed());
        }
        let ticket = Self {
            number: number.parse().map_err(|_| malformed())?,
            nonce: Nonce::from_hex(nonce).map_err(|_| malformed())?,
        };
        if ticket.to_string() != raw {
            return Err(malformed());
        }
        Ok(ticket)
    }

    pub const fn number(&self) -> u64 {
        self.number
    }

    pub const fn is_ahead_of(&self, other: &Self) -> bool {
        self.number < other.number
    }
}

impl fmt::Display for Ticket {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}-{}", self.number, self.nonce)
    }
}

/// The arguments a ticket was issued for, which every re-presentation must
/// repeat.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Binding {
    Search { query: ArxivQuery, limit: Limit },
    Lookup(ArxivId),
}

/// An argument a re-presentation can get wrong.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Argument {
    Verb,
    Query,
    Id,
    Limit,
}

impl Argument {
    pub const fn code(self) -> &'static str {
        match self {
            Self::Verb => "verb",
            Self::Query => "query",
            Self::Id => "id",
            Self::Limit => "--limit",
        }
    }
}

impl Binding {
    pub fn of(request: &ArxivRequest) -> Self {
        match request {
            ArxivRequest::Search { query, limit } => Self::Search {
                query: query.clone(),
                limit: *limit,
            },
            ArxivRequest::Lookup(id) => Self::Lookup(id.clone()),
        }
    }

    /// A differing verb is reported alone: the other arguments of a search
    /// and a lookup are not comparable.
    pub fn differing(&self, other: &Self) -> Vec<Argument> {
        match (self, other) {
            (
                Self::Search { query, limit },
                Self::Search {
                    query: other_query,
                    limit: other_limit,
                },
            ) => [
                (query != other_query).then_some(Argument::Query),
                (limit != other_limit).then_some(Argument::Limit),
            ]
            .into_iter()
            .flatten()
            .collect(),
            (Self::Lookup(id), Self::Lookup(other_id)) => (id != other_id)
                .then_some(Argument::Id)
                .into_iter()
                .collect(),
            _ => vec![Argument::Verb],
        }
    }

    /// The value as the caller spelled it, or empty for an argument this
    /// verb does not take.
    pub fn value_of(&self, argument: Argument) -> String {
        match (self, argument) {
            (Self::Search { .. }, Argument::Verb) => "search".to_owned(),
            (Self::Lookup(_), Argument::Verb) => "lookup".to_owned(),
            (Self::Search { query, .. }, Argument::Query) => {
                query.spelled().to_owned()
            }
            (Self::Search { limit, .. }, Argument::Limit) => {
                limit.get().to_string()
            }
            (Self::Lookup(id), Argument::Id) => id.to_string(),
            _ => String::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Presence {
    /// Some call holds the ticket, or whether one does cannot be told.
    Live,
    Absent {
        ended_at: Option<SystemTime>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Standing {
    pub ticket: Ticket,
    pub binding: Binding,
    pub issued_at: SystemTime,
    pub presented_at: SystemTime,
    pub last_retryable: Option<Reason>,
    pub presence: Presence,
}

/// How many tickets wait ahead of one, plus one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position(u32);

impl Position {
    pub const fn new(position: u32) -> Self {
        Self(position)
    }

    pub const fn get(self) -> u32 {
        self.0
    }
}

/// What the front rule needs to know of a ticket ahead.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Contender {
    pub is_live: bool,
    pub issued_at: SystemTime,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TicketRejection {
    Mismatch {
        ticket: Ticket,
        issued: Binding,
        differing: Vec<Argument>,
    },
    AlreadyLive(Ticket),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Presentation {
    Join,
    Resume(Ticket),
    Rejected(TicketRejection),
    OverCap {
        ticket: Ticket,
        last_retryable: Option<Reason>,
    },
}

/// A snapshot of every ticket on record. `unparsed` holds the tickets of
/// records that could not be read, which still take part in numbering.
///
/// Every elapsed time saturates at zero, so a stamp later than `now` reads
/// as just stamped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Queue {
    standings: Vec<Standing>,
    unparsed: Vec<Ticket>,
}

impl Queue {
    pub const fn new(standings: Vec<Standing>, unparsed: Vec<Ticket>) -> Self {
        Self {
            standings,
            unparsed,
        }
    }

    /// The cap is judged only here, so a ticket that crosses it while its
    /// call is waiting is still served.
    pub fn present(
        &self,
        presented: Option<&Ticket>,
        binding: &Binding,
        now: SystemTime,
    ) -> Presentation {
        let Some(standing) = presented.and_then(|presented| {
            self.standings
                .iter()
                .find(|standing| standing.ticket == *presented)
        }) else {
            return Presentation::Join;
        };
        if is_expired(standing, now) {
            return Presentation::Join;
        }
        let differing = standing.binding.differing(binding);
        if !differing.is_empty() {
            return Presentation::Rejected(TicketRejection::Mismatch {
                ticket: standing.ticket.clone(),
                issued: standing.binding.clone(),
                differing,
            });
        }
        if end_of(standing, now).is_none() {
            return Presentation::Rejected(TicketRejection::AlreadyLive(
                standing.ticket.clone(),
            ));
        }
        if elapsed(standing.issued_at, now) > CAP {
            return Presentation::OverCap {
                ticket: standing.ticket.clone(),
                last_retryable: standing.last_retryable,
            };
        }
        Presentation::Resume(standing.ticket.clone())
    }

    pub fn position_of(&self, ticket: &Ticket, now: SystemTime) -> Position {
        let ahead = self
            .standings
            .iter()
            .filter(|standing| standing.ticket.is_ahead_of(ticket))
            .filter(|standing| !is_expired(standing, now))
            .count();
        Position(u32::try_from(ahead).unwrap_or(u32::MAX).saturating_add(1))
    }

    pub fn next_number(&self) -> u64 {
        self.standings
            .iter()
            .map(|standing| &standing.ticket)
            .chain(&self.unparsed)
            .map(Ticket::number)
            .max()
            .map_or(1, |largest| largest.saturating_add(1))
    }

    pub fn expired(&self, now: SystemTime) -> Vec<Ticket> {
        self.standings
            .iter()
            .filter(|standing| is_expired(standing, now))
            .map(|standing| standing.ticket.clone())
            .collect()
    }

    /// The end to record for each absent ticket whose call never recorded
    /// one: the latest its call could have ended.
    pub fn stamps(&self, now: SystemTime) -> Vec<(Ticket, SystemTime)> {
        self.standings
            .iter()
            .filter(|standing| {
                standing.presence == Presence::Absent { ended_at: None }
            })
            .map(|standing| {
                (standing.ticket.clone(), latest_end(standing, now))
            })
            .collect()
    }
}

/// The one home of the front rule: a ticket is front unless a contender
/// ahead of it is live and not abandoned. Stops at the first that is, so a
/// lazy `ahead` reads no further.
pub fn is_front_given(
    ahead: impl IntoIterator<Item = Contender>,
    now: SystemTime,
) -> bool {
    !ahead.into_iter().any(|contender| {
        contender.is_live && !is_abandoned(contender.issued_at, now)
    })
}

pub fn is_abandoned(issued_at: SystemTime, now: SystemTime) -> bool {
    elapsed(issued_at, now) > ABANDONED_AFTER
}

fn elapsed(since: SystemTime, now: SystemTime) -> Duration {
    now.duration_since(since).unwrap_or(Duration::ZERO)
}

fn is_expired(standing: &Standing, now: SystemTime) -> bool {
    end_of(standing, now).is_some_and(|end| elapsed(end, now) > EXPIRY)
}

/// When the ticket's last call ended, or `None` while it is live.
/// Abandonment only ever brings an end earlier.
fn end_of(standing: &Standing, now: SystemTime) -> Option<SystemTime> {
    let recorded = match standing.presence {
        Presence::Absent {
            ended_at: Some(ended_at),
        } => Some(ended_at),
        Presence::Absent { ended_at: None } => Some(latest_end(standing, now)),
        Presence::Live => None,
    };
    if !is_abandoned(standing.issued_at, now) {
        return recorded;
    }
    let abandoned_at = standing.issued_at + ABANDONED_AFTER;
    Some(recorded.map_or(abandoned_at, |end| end.min(abandoned_at)))
}

fn latest_end(standing: &Standing, now: SystemTime) -> SystemTime {
    now.min(standing.presented_at + LONGEST_INVOCATION)
}

/// Where an arXiv call waits its turn across invocations.
pub trait ArxivQueue {
    /// Presents `presented`, or no ticket, for the call bound to `binding`.
    /// Waits for the queue no longer than leaves the deadline a serving
    /// window of `spacing` and one request.
    fn join(
        &self,
        binding: &Binding,
        presented: Option<&Ticket>,
        deadline: &Deadline,
        spacing: Duration,
    ) -> Joining<'_>;
}

pub enum Joining<'q> {
    Queued(Box<dyn Place + 'q>),
    /// The queue could not be used, so the call has no place to keep.
    Unqueued,
    Rejected(TicketRejection),
    OverCap {
        ticket: Ticket,
        last_retryable: Option<Reason>,
    },
}

/// A ticket held live by this call until it steps aside or leaves.
pub trait Place {
    fn ticket(&self) -> &Ticket;

    fn is_front(&self) -> bool;

    /// Keeps the ticket's place for a re-presentation.
    fn step_aside(
        self: Box<Self>,
        last_retryable: Option<Reason>,
        deadline: &Deadline,
    ) -> Position;

    /// Gives the place up for good.
    fn leave(self: Box<Self>, deadline: &Deadline);
}
