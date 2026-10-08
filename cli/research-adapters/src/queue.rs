//! The arXiv fetch queue on disk: one record and one lock file per ticket,
//! and a bookkeeping lock every write happens under.
//!
//! A ticket is live while its call holds an exclusive `flock` on the
//! ticket's lock file. Probes take only a momentary shared lock, so probers
//! never block each other and only an owner reads as held.

use std::cell::Cell;
use std::cell::RefCell;
use std::cmp::Reverse;
use std::collections::HashMap;
use std::collections::HashSet;
use std::fs::File;
use std::fs::OpenOptions;
use std::io::ErrorKind;
use std::rc::Rc;
use std::time::Duration;
use std::time::SystemTime;

use research::sources::classify::Reason;
use research::sources::fetch::Clock;
use research::sources::queue::is_abandoned;
use research::sources::queue::is_front_given;
use research::sources::queue::ArxivQueue;
use research::sources::queue::Binding;
use research::sources::queue::Contender;
use research::sources::queue::Joining;
use research::sources::queue::Nonce;
use research::sources::queue::Place;
use research::sources::queue::Position;
use research::sources::queue::Presence;
use research::sources::queue::Presentation;
use research::sources::queue::Queue;
use research::sources::queue::Standing;
use research::sources::queue::Ticket;
use research::sources::queue::LONGEST_INVOCATION;
use research::sources::request::ArxivId;
use research::sources::request::ArxivQuery;
use research::sources::request::Limit;
use research::sources::schedule::Deadline;
use rustix::fs::flock;
use rustix::fs::FlockOperation;
use rustix::io::Errno;
use serde::Deserialize;
use serde::Serialize;
use store::TEMP_PREFIX;

use crate::clock::millis_since_epoch;
use crate::diagnostics::Diagnostics;
use crate::scratch::ScratchDir;

const QUEUE_LOCK: &str = "queue.lock";
const SCHEMA_VERSION: u32 = 1;

const BOOKKEEPING_RETRY: Duration = Duration::from_millis(10);
/// Even a heavily descheduled prober releases its shared lock within this;
/// one held longer belongs to a stuck process.
const OWN_PATIENCE: Duration = Duration::from_millis(500);
const OWN_RETRY: Duration = Duration::from_millis(1);

pub fn random_nonce() -> Nonce {
    Nonce::from_bits(rand::random())
}

pub struct FileArxivQueue {
    scratch: ScratchDir,
    clock: Rc<dyn Clock>,
    diagnostics: Rc<dyn Diagnostics>,
    nonces: Box<dyn Fn() -> Nonce>,
    unprobeable_reported: RefCell<HashSet<Ticket>>,
}

impl FileArxivQueue {
    pub fn new(
        scratch: ScratchDir,
        clock: Rc<dyn Clock>,
        diagnostics: Rc<dyn Diagnostics>,
        nonces: Box<dyn Fn() -> Nonce>,
    ) -> Self {
        Self {
            scratch,
            clock,
            diagnostics,
            nonces,
            unprobeable_reported: RefCell::default(),
        }
    }

    /// The bookkeeping lock, retried while `bound` allows; `None` when the
    /// queue cannot be locked in time.
    fn bookkept(&self, deadline: &Deadline, bound: Bound) -> Option<File> {
        let lock = match self.scratch.open(QUEUE_LOCK) {
            Ok(lock) => lock,
            Err(error) => {
                self.report(&error);
                return None;
            }
        };
        loop {
            match flock(&lock, FlockOperation::NonBlockingLockExclusive) {
                Ok(()) => return Some(lock),
                Err(Errno::WOULDBLOCK) => {
                    if !bound.allows(deadline, self.clock.now()) {
                        self.report(&format!(
                            "{} was held past this call's budget",
                            self.scratch.path(QUEUE_LOCK).display()
                        ));
                        return None;
                    }
                    self.clock.sleep(BOOKKEEPING_RETRY);
                }
                Err(errno) => {
                    self.report(&format!(
                        "could not lock {}: {errno}",
                        self.scratch.path(QUEUE_LOCK).display()
                    ));
                    return None;
                }
            }
        }
    }

    fn snapshot(&self) -> Result<Snapshot, String> {
        let names = self.scratch.names()?;
        let mut snapshot = Snapshot::default();
        let mut lock_tickets = Vec::new();
        for name in names {
            if name.starts_with(TEMP_PREFIX) {
                snapshot.temp_files.push(name);
            } else if let Some(ticket) = ticket_named(&name, RECORD) {
                match self.read_record(&ticket) {
                    Some(record) => snapshot.records.push((ticket, record)),
                    None => snapshot.set_aside.push(ticket),
                }
            } else if let Some(ticket) = ticket_named(&name, LOCK) {
                lock_tickets.push(ticket);
            }
        }
        snapshot.orphan_locks = lock_tickets
            .into_iter()
            .filter(|ticket| !snapshot.has_record_file(ticket))
            .collect();
        Ok(snapshot)
    }

    fn queue_of(&self, snapshot: &Snapshot) -> Queue {
        let standings = snapshot
            .records
            .iter()
            .filter_map(|(ticket, record)| {
                record
                    .standing(ticket.clone(), self.presence_of(ticket, record))
            })
            .collect();
        Queue::new(standings, snapshot.set_aside.clone())
    }

    fn presence_of(&self, ticket: &Ticket, record: &Record) -> Presence {
        match self.probe(ticket) {
            Probe::Free => Presence::Absent {
                ended_at: record.ended_ms.map(at_millis),
            },
            Probe::Held => Presence::Live,
            Probe::Unknown => Presence::Unknown,
        }
    }

    fn probe(&self, ticket: &Ticket) -> Probe {
        self.try_probe(ticket).unwrap_or_else(|error| {
            if self
                .unprobeable_reported
                .borrow_mut()
                .insert(ticket.clone())
            {
                self.report(&format!(
                    "{error}; counting its ticket live until one invocation \
                     after its presentation"
                ));
            }
            Probe::Unknown
        })
    }

    /// Opened for writing without creating, so a lock path that is not a
    /// regular file reads as unknown rather than free.
    fn try_probe(&self, ticket: &Ticket) -> Result<Probe, String> {
        let path = self.scratch.path(&lock_name(ticket));
        let file = match OpenOptions::new().read(true).write(true).open(&path) {
            Ok(file) => file,
            Err(error) if error.kind() == ErrorKind::NotFound => {
                return Ok(Probe::Free)
            }
            Err(error) => {
                return Err(format!(
                    "could not open {}: {error}",
                    path.display()
                ))
            }
        };
        match flock(&file, FlockOperation::NonBlockingLockShared) {
            Ok(()) => {
                let _ = flock(&file, FlockOperation::Unlock);
                Ok(Probe::Free)
            }
            Err(Errno::WOULDBLOCK) => Ok(Probe::Held),
            Err(errno) => {
                Err(format!("could not probe {}: {errno}", path.display()))
            }
        }
    }

    /// The owner's exclusive lock. A prober holds its shared lock only for
    /// an instant, so a lock still held after `OWN_PATIENCE` is given up on.
    fn own(
        &self,
        ticket: &Ticket,
        deadline: &Deadline,
        spacing: Duration,
    ) -> Option<File> {
        let lock = match self.scratch.open(&lock_name(ticket)) {
            Ok(lock) => lock,
            Err(error) => {
                self.report(&error);
                return None;
            }
        };
        let started = self.clock.now();
        loop {
            match flock(&lock, FlockOperation::NonBlockingLockExclusive) {
                Ok(()) => return Some(lock),
                Err(Errno::WOULDBLOCK) => {
                    let now = self.clock.now();
                    let patient = now.duration_since(started) < OWN_PATIENCE;
                    if !patient
                        || !deadline
                            .admits_attempt_after(now, spacing + OWN_RETRY)
                    {
                        self.report(&format!(
                            "{} stayed locked by another process",
                            self.scratch.path(&lock_name(ticket)).display()
                        ));
                        return None;
                    }
                    self.clock.sleep(OWN_RETRY);
                }
                Err(errno) => {
                    self.report(&format!(
                        "could not lock {}: {errno}",
                        self.scratch.path(&lock_name(ticket)).display()
                    ));
                    return None;
                }
            }
        }
    }

    /// Stamps, prunes and removes what the snapshot shows is owed. Each
    /// failure is reported and the pass goes on.
    fn housekeeping(
        &self,
        snapshot: &Snapshot,
        queue: &Queue,
        now: SystemTime,
    ) {
        for (ticket, ended_at) in queue.stamps(now) {
            if let Some(record) = snapshot.record(&ticket) {
                self.write_record(
                    &ticket,
                    &Record {
                        ended_ms: Some(millis_since_epoch(ended_at)),
                        ..record.clone()
                    },
                );
            }
        }
        for ticket in queue.expired(now) {
            self.remove_ticket(&ticket);
        }
        for ticket in &snapshot.set_aside {
            let name = record_name(ticket);
            let abandoned = self
                .scratch
                .modified(&name)
                .is_some_and(|modified| is_abandoned(modified, now));
            if self.probe(ticket) == Probe::Free || abandoned {
                self.report(&format!(
                    "removed unreadable {}",
                    self.scratch.path(&name).display()
                ));
                self.remove_ticket(ticket);
            } else {
                self.report(&format!(
                    "kept unreadable {} while its ticket may be live",
                    self.scratch.path(&name).display()
                ));
            }
        }
        for ticket in &snapshot.orphan_locks {
            if self.probe(ticket) == Probe::Free {
                self.remove(&lock_name(ticket));
            }
        }
        for name in &snapshot.temp_files {
            let stale = self.scratch.modified(name).is_some_and(|modified| {
                now.duration_since(modified)
                    .is_ok_and(|age| age > LONGEST_INVOCATION)
            });
            if stale {
                self.remove(name);
            }
        }
    }

    fn take_place(
        &self,
        ticket: Ticket,
        record: &Record,
        queue: &Queue,
        deadline: &Deadline,
        spacing: Duration,
    ) -> Joining<'_> {
        let Some(lock) = self.own(&ticket, deadline, spacing) else {
            return Joining::Unqueued;
        };
        if !self.write_record(&ticket, record) {
            return Joining::Unqueued;
        }
        let position = queue.position_of(&ticket, self.clock.wall_now());
        Joining::Queued(Box::new(FilePlace {
            queue: self,
            ticket,
            _lock: lock,
            position,
            issue_times: RefCell::default(),
            listing_reported: Cell::new(false),
        }))
    }

    fn read_record(&self, ticket: &Ticket) -> Option<Record> {
        self.scratch
            .read(&record_name(ticket))
            .and_then(|bytes| serde_json::from_slice::<Record>(&bytes).ok())
            .filter(|record| record.schema_version == SCHEMA_VERSION)
            .filter(|record| {
                record.standing(ticket.clone(), Presence::Live).is_some()
            })
    }

    fn write_record(&self, ticket: &Ticket, record: &Record) -> bool {
        let written = serde_json::to_vec(record)
            .map_err(|error| error.to_string())
            .and_then(|bytes| {
                self.scratch
                    .replace_without_sync(&record_name(ticket), &bytes)
            });
        written.map_err(|error| self.report(&error)).is_ok()
    }

    fn remove_ticket(&self, ticket: &Ticket) {
        self.remove(&record_name(ticket));
        self.remove(&lock_name(ticket));
    }

    fn remove(&self, name: &str) {
        if let Err(error) = self.scratch.remove(name) {
            self.report(&error);
        }
    }

    fn report(&self, detail: &str) {
        self.diagnostics
            .report(&format!("research fetch: arxiv queue {detail}"));
    }
}

impl ArxivQueue for FileArxivQueue {
    fn join(
        &self,
        binding: &Binding,
        presented: Option<&Ticket>,
        deadline: &Deadline,
        spacing: Duration,
    ) -> Joining<'_> {
        let Some(_queue_lock) =
            self.bookkept(deadline, Bound::ServingWindow(spacing))
        else {
            return Joining::Unqueued;
        };
        let snapshot = match self.snapshot() {
            Ok(snapshot) => snapshot,
            Err(error) => {
                self.report(&error);
                return Joining::Unqueued;
            }
        };
        let queue = self.queue_of(&snapshot);
        let now = self.clock.wall_now();
        match queue.present(presented, binding, now) {
            Presentation::Rejected(rejection) => Joining::Rejected(rejection),
            Presentation::OverCap {
                ticket,
                last_retryable,
            } => {
                self.housekeeping(&snapshot, &queue, now);
                self.remove_ticket(&ticket);
                Joining::OverCap {
                    ticket,
                    last_retryable,
                }
            }
            Presentation::Join => {
                self.housekeeping(&snapshot, &queue, now);
                let ticket =
                    Ticket::issue(queue.next_number(), (self.nonces)());
                let record = Record::issued(binding, now);
                self.take_place(ticket, &record, &queue, deadline, spacing)
            }
            Presentation::Resume(ticket) => {
                self.housekeeping(&snapshot, &queue, now);
                let Some(issued) = snapshot.record(&ticket) else {
                    return Joining::Unqueued;
                };
                let record = issued.presented_at(now);
                let position = queue.position_of(&ticket, now);
                match self.take_place(
                    ticket.clone(),
                    &record,
                    &queue,
                    deadline,
                    spacing,
                ) {
                    Joining::Unqueued => Joining::Unheld { ticket, position },
                    joining => joining,
                }
            }
        }
    }
}

struct FilePlace<'q> {
    queue: &'q FileArxivQueue,
    ticket: Ticket,
    _lock: File,
    position: Position,
    issue_times: RefCell<HashMap<Ticket, SystemTime>>,
    listing_reported: Cell<bool>,
}

impl FilePlace<'_> {
    /// A ticket ahead as the front rule sees it. One whose files vanish
    /// mid-read has just left, so it is not live.
    fn contender(&self, ticket: &Ticket, now: SystemTime) -> Contender {
        let absent = Contender {
            presence: Presence::Absent { ended_at: None },
            issued_at: now,
            presented_at: now,
        };
        let presence = match self.queue.probe(ticket) {
            Probe::Free => return absent,
            Probe::Held => Presence::Live,
            Probe::Unknown => Presence::Unknown,
        };
        let Some(issued_at) = self.issued_at(ticket) else {
            return absent;
        };
        let presented_at = match presence {
            Presence::Unknown => self.presented_at(ticket).unwrap_or(now),
            _ => issued_at,
        };
        Contender {
            presence,
            issued_at,
            presented_at,
        }
    }

    /// Read afresh, as a resumption moves it. A record too corrupt to say
    /// falls back to its last write, which is never before a presentation.
    fn presented_at(&self, ticket: &Ticket) -> Option<SystemTime> {
        let name = record_name(ticket);
        self.queue
            .scratch
            .read(&name)
            .and_then(|bytes| {
                serde_json::from_slice::<PresentedOnly>(&bytes).ok()
            })
            .map(|presented| at_millis(presented.presented_ms))
            .or_else(|| self.queue.scratch.modified(&name))
    }

    fn issued_at(&self, ticket: &Ticket) -> Option<SystemTime> {
        if let Some(issued_at) = self.issue_times.borrow().get(ticket) {
            return Some(*issued_at);
        }
        let issued_at = self
            .queue
            .scratch
            .read(&record_name(ticket))
            .and_then(|bytes| serde_json::from_slice::<IssuedOnly>(&bytes).ok())
            .map(|issued| at_millis(issued.issued_ms))
            .or_else(|| self.queue.scratch.modified(&record_name(ticket)))?;
        self.issue_times
            .borrow_mut()
            .insert(ticket.clone(), issued_at);
        Some(issued_at)
    }

    fn report_unlistable_once(&self, error: &str) {
        if !self.listing_reported.replace(true) {
            self.queue
                .report(&format!("{error}; serving without fairness"));
        }
    }
}

impl Place for FilePlace<'_> {
    fn ticket(&self) -> &Ticket {
        &self.ticket
    }

    /// Takes no lock and writes nothing, so asking never holds the queue up.
    fn is_front(&self) -> bool {
        let names = match self.queue.scratch.names() {
            Ok(names) => names,
            Err(error) => {
                self.report_unlistable_once(&error);
                return true;
            }
        };
        let mut ahead: Vec<Ticket> = names
            .iter()
            .filter_map(|name| ticket_named(name, RECORD))
            .filter(|ticket| ticket.is_ahead_of(&self.ticket))
            .collect();
        ahead.sort_by_key(|ticket| Reverse(ticket.number()));
        let now = self.queue.clock.wall_now();
        is_front_given(
            ahead.iter().map(|ticket| self.contender(ticket, now)),
            now,
        )
    }

    fn step_aside(
        self: Box<Self>,
        last_retryable: Option<Reason>,
        deadline: &Deadline,
    ) -> Position {
        let Some(_queue_lock) = self.queue.bookkept(deadline, Bound::Deadline)
        else {
            return self.position;
        };
        let snapshot = match self.queue.snapshot() {
            Ok(snapshot) => snapshot,
            Err(error) => {
                self.queue.report(&error);
                return self.position;
            }
        };
        let queue = self.queue.queue_of(&snapshot);
        let now = self.queue.clock.wall_now();
        self.queue.housekeeping(&snapshot, &queue, now);
        if let Some(record) = snapshot.record(&self.ticket) {
            self.queue.write_record(
                &self.ticket,
                &Record {
                    ended_ms: Some(millis_since_epoch(now)),
                    last_retryable: last_retryable
                        .map(|reason| reason.code().to_owned()),
                    ..record.clone()
                },
            );
        }
        queue.position_of(&self.ticket, now)
    }

    fn leave(self: Box<Self>, deadline: &Deadline) {
        let Some(_queue_lock) = self.queue.bookkept(deadline, Bound::Deadline)
        else {
            return;
        };
        match self.queue.snapshot() {
            Ok(snapshot) => {
                let queue = self.queue.queue_of(&snapshot);
                let now = self.queue.clock.wall_now();
                self.queue.housekeeping(&snapshot, &queue, now);
            }
            Err(error) => self.queue.report(&error),
        }
        self.queue.remove_ticket(&self.ticket);
    }
}

#[derive(Clone, Copy)]
enum Bound {
    /// Leave the call a serving window of this spacing and one request.
    ServingWindow(Duration),
    Deadline,
}

impl Bound {
    fn allows(self, deadline: &Deadline, now: std::time::Instant) -> bool {
        match self {
            Self::ServingWindow(spacing) => {
                deadline.admits_attempt_after(now, spacing + BOOKKEEPING_RETRY)
            }
            Self::Deadline => deadline.remaining(now) > BOOKKEEPING_RETRY,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Probe {
    Free,
    Held,
    Unknown,
}

#[derive(Default)]
struct Snapshot {
    records: Vec<(Ticket, Record)>,
    set_aside: Vec<Ticket>,
    orphan_locks: Vec<Ticket>,
    temp_files: Vec<String>,
}

impl Snapshot {
    fn record(&self, ticket: &Ticket) -> Option<&Record> {
        self.records
            .iter()
            .find(|(recorded, _)| recorded == ticket)
            .map(|(_, record)| record)
    }

    fn has_record_file(&self, ticket: &Ticket) -> bool {
        self.record(ticket).is_some() || self.set_aside.contains(ticket)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Record {
    schema_version: u32,
    issued_ms: u64,
    presented_ms: u64,
    ended_ms: Option<u64>,
    last_retryable: Option<String>,
    #[serde(flatten)]
    binding: StoredBinding,
}

#[derive(Deserialize)]
struct IssuedOnly {
    issued_ms: u64,
}

#[derive(Deserialize)]
struct PresentedOnly {
    presented_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "verb", rename_all = "lowercase")]
enum StoredBinding {
    Search { query: String, limit: u8 },
    Lookup { id: String },
}

impl Record {
    fn issued(binding: &Binding, now: SystemTime) -> Self {
        let now = millis_since_epoch(now);
        Self {
            schema_version: SCHEMA_VERSION,
            issued_ms: now,
            presented_ms: now,
            ended_ms: None,
            last_retryable: None,
            binding: StoredBinding::from(binding),
        }
    }

    /// A resumed ticket clears its reason too, so a later lost
    /// `step_aside` write cannot report a stale one.
    fn presented_at(&self, now: SystemTime) -> Self {
        Self {
            presented_ms: millis_since_epoch(now),
            ended_ms: None,
            last_retryable: None,
            ..self.clone()
        }
    }

    fn standing(&self, ticket: Ticket, presence: Presence) -> Option<Standing> {
        let last_retryable = match &self.last_retryable {
            Some(code) => Some(Reason::from_code(code)?),
            None => None,
        };
        Some(Standing {
            ticket,
            binding: self.binding.to_binding()?,
            issued_at: at_millis(self.issued_ms),
            presented_at: at_millis(self.presented_ms),
            last_retryable,
            presence,
        })
    }
}

impl From<&Binding> for StoredBinding {
    fn from(binding: &Binding) -> Self {
        match binding {
            Binding::Search { query, limit } => Self::Search {
                query: query.spelled().to_owned(),
                limit: limit.get(),
            },
            Binding::Lookup(id) => Self::Lookup { id: id.to_string() },
        }
    }
}

impl StoredBinding {
    fn to_binding(&self) -> Option<Binding> {
        Some(match self {
            Self::Search { query, limit } => Binding::Search {
                query: ArxivQuery::parse(query).ok()?,
                limit: Limit::try_from(*limit).ok()?,
            },
            Self::Lookup { id } => Binding::Lookup(ArxivId::parse(id).ok()?),
        })
    }
}

const RECORD: &str = ".json";
const LOCK: &str = ".lock";

fn record_name(ticket: &Ticket) -> String {
    format!("{ticket}{RECORD}")
}

fn lock_name(ticket: &Ticket) -> String {
    format!("{ticket}{LOCK}")
}

fn ticket_named(name: &str, suffix: &str) -> Option<Ticket> {
    name.strip_suffix(suffix)
        .and_then(|stem| Ticket::parse(stem).ok())
}

fn at_millis(millis: u64) -> SystemTime {
    SystemTime::UNIX_EPOCH + Duration::from_millis(millis)
}
