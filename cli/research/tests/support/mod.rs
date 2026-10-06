#![allow(dead_code)]

use std::cell::Cell;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;
use std::time::Duration;
use std::time::Instant;
use std::time::SystemTime;

use research::sources::arxiv::Entry;
use research::sources::arxiv::RawVersion;
use research::sources::classify::Received;
use research::sources::classify::Response;
use research::sources::fetch::ArxivDecoder;
use research::sources::fetch::Attempted;
use research::sources::fetch::Clock;
use research::sources::fetch::ConfirmationCache;
use research::sources::fetch::Contention;
use research::sources::fetch::ContentionLog;
use research::sources::fetch::DecodeFailure;
use research::sources::fetch::OpenAlexDecoder;
use research::sources::fetch::PacingGate;
use research::sources::fetch::ServingTurn;
use research::sources::fetch::Transport;
use research::sources::fetch::WaitTooLong;
use research::sources::openalex::Work;
use research::sources::request::ArxivId;
use research::sources::request::UpstreamRequest;
use research::sources::schedule::Deadline;

pub const fn secs(seconds: u64) -> Duration {
    Duration::from_secs(seconds)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    Slept(Duration),
    Served,
    Attempt,
    Released,
    SteppedAside,
    Left,
}

/// One ordered log several doubles write to, so a test can assert how their
/// events interleave.
#[derive(Clone, Default)]
pub struct Timeline(Rc<RefCell<Vec<Event>>>);

impl Timeline {
    pub fn push(&self, event: Event) {
        self.0.borrow_mut().push(event);
    }

    pub fn events(&self) -> Vec<Event> {
        self.0.borrow().clone()
    }
}

/// A virtual timeline: sleeping advances it instead of waiting, and every
/// wait is recorded.
pub struct RecordingClock {
    origin: Instant,
    wall_origin: SystemTime,
    elapsed: Cell<Duration>,
    slept: RefCell<Vec<Duration>>,
    timeline: Timeline,
}

impl RecordingClock {
    pub fn new() -> Self {
        Self::on(Timeline::default())
    }

    pub fn on(timeline: Timeline) -> Self {
        Self {
            origin: Instant::now(),
            wall_origin: SystemTime::UNIX_EPOCH + secs(1_800_000_000),
            elapsed: Cell::new(Duration::ZERO),
            slept: RefCell::new(Vec::new()),
            timeline,
        }
    }

    /// Spends the deadline down so only `left` of it remains.
    pub fn leaving(&self, left: Duration) {
        self.advance(secs(100) - left);
    }

    pub fn advance(&self, by: Duration) {
        self.elapsed.set(self.elapsed.get() + by);
    }

    pub fn slept(&self) -> Vec<Duration> {
        self.slept.borrow().clone()
    }

    pub fn wall_at(&self, offset: Duration) -> SystemTime {
        self.wall_origin + offset
    }

    pub fn deadline(&self) -> Deadline {
        Deadline::starting(self.origin, secs(100), secs(30))
    }
}

impl Clock for RecordingClock {
    fn now(&self) -> Instant {
        self.origin + self.elapsed.get()
    }

    fn wall_now(&self) -> SystemTime {
        self.wall_origin + self.elapsed.get()
    }

    fn sleep(&self, duration: Duration) {
        self.slept.borrow_mut().push(duration);
        self.timeline.push(Event::Slept(duration));
        self.advance(duration);
    }
}

/// Answers each request with the next scripted response, repeating the last
/// once the script runs out, and advances the clock by how long each took.
pub struct ScriptedTransport<'a> {
    clock: &'a RecordingClock,
    script: RefCell<VecDeque<(Response, Duration)>>,
    last: RefCell<Option<(Response, Duration)>>,
    sent: RefCell<Vec<UpstreamRequest>>,
}

impl<'a> ScriptedTransport<'a> {
    pub fn new(
        clock: &'a RecordingClock,
        script: Vec<(Response, Duration)>,
    ) -> Self {
        Self {
            clock,
            script: RefCell::new(script.into()),
            last: RefCell::new(None),
            sent: RefCell::new(Vec::new()),
        }
    }

    pub fn immediate(
        clock: &'a RecordingClock,
        responses: Vec<Response>,
    ) -> Self {
        Self::new(
            clock,
            responses
                .into_iter()
                .map(|response| (response, Duration::ZERO))
                .collect(),
        )
    }

    pub fn hits(&self) -> usize {
        self.sent.borrow().len()
    }

    pub fn urls(&self) -> Vec<String> {
        self.sent
            .borrow()
            .iter()
            .map(|request| request.url().to_owned())
            .collect()
    }

    pub fn bearers(&self) -> Vec<Option<String>> {
        self.sent
            .borrow()
            .iter()
            .map(|request| request.bearer().map(str::to_owned))
            .collect()
    }
}

impl Transport for ScriptedTransport<'_> {
    fn send(&self, request: &UpstreamRequest) -> Response {
        self.sent.borrow_mut().push(request.clone());
        if let Some(step) = self.script.borrow_mut().pop_front() {
            *self.last.borrow_mut() = Some(step);
        }
        let (response, took) = self
            .last
            .borrow()
            .clone()
            .unwrap_or((Response::ConnectionFailed, Duration::ZERO));
        self.clock.advance(took);
        response
    }
}

pub fn status(code: u16) -> Response {
    Response::Received(Received::status(code))
}

pub fn body(content: &[u8]) -> Response {
    Response::Received(Received {
        body: content.to_vec(),
        ..Received::status(200)
    })
}

pub fn retry_after(code: u16, seconds: u64) -> Response {
    Response::Received(Received {
        retry_after: Some(secs(seconds)),
        ..Received::status(code)
    })
}

/// Serves a turn at once unless scripted to stay busy, runs every attempt
/// unless scripted to refuse one, and records each deferral the domain asks
/// it to persist.
#[derive(Default)]
pub struct RecordingGate {
    spacing: Duration,
    busy_for: Cell<usize>,
    refuse_from_attempt: Option<usize>,
    turns: Cell<usize>,
    attempts: Cell<usize>,
    deferrals: RefCell<Vec<Option<SystemTime>>>,
    serving: Cell<bool>,
    timeline: Timeline,
}

impl RecordingGate {
    pub fn on(timeline: Timeline) -> Self {
        Self {
            timeline,
            ..Self::default()
        }
    }

    #[must_use]
    pub const fn spacing(mut self, spacing: Duration) -> Self {
        self.spacing = spacing;
        self
    }

    /// Answers `try_serve` with `None` for the next `tries` calls.
    #[must_use]
    pub fn busy_for(self, tries: usize) -> Self {
        self.busy_for.set(tries);
        self
    }

    #[must_use]
    pub fn always_busy(self) -> Self {
        self.busy_for(usize::MAX)
    }

    /// Refuses the `attempt`th attempt, and every one after it, as a wait
    /// too long for the deadline.
    #[must_use]
    pub const fn refusing_from(mut self, attempt: usize) -> Self {
        self.refuse_from_attempt = Some(attempt);
        self
    }

    pub const fn turns_served(&self) -> usize {
        self.turns.get()
    }

    pub const fn attempts(&self) -> usize {
        self.attempts.get()
    }

    pub fn deferrals(&self) -> Vec<Option<SystemTime>> {
        self.deferrals.borrow().clone()
    }

    pub const fn is_serving(&self) -> bool {
        self.serving.get()
    }
}

impl PacingGate for RecordingGate {
    fn spacing(&self) -> Duration {
        self.spacing
    }

    fn try_serve(&self) -> Option<Box<dyn ServingTurn + '_>> {
        let busy = self.busy_for.get();
        if busy == usize::MAX {
            return None;
        }
        if busy > 0 {
            self.busy_for.set(busy - 1);
            return None;
        }
        self.turns.set(self.turns.get() + 1);
        self.serving.set(true);
        self.timeline.push(Event::Served);
        Some(Box::new(RecordingTurn { gate: self }))
    }
}

struct RecordingTurn<'g> {
    gate: &'g RecordingGate,
}

impl ServingTurn for RecordingTurn<'_> {
    fn paced(
        &self,
        attempt: &mut dyn FnMut() -> Attempted,
        _deadline: &Deadline,
    ) -> Result<(), WaitTooLong> {
        let gate = self.gate;
        let number = gate.attempts.get() + 1;
        if gate.refuse_from_attempt.is_some_and(|from| number >= from) {
            return Err(WaitTooLong);
        }
        gate.attempts.set(number);
        gate.timeline.push(Event::Attempt);
        let attempted = attempt();
        gate.deferrals.borrow_mut().push(attempted.defer_until);
        Ok(())
    }
}

impl Drop for RecordingTurn<'_> {
    fn drop(&mut self) {
        self.gate.serving.set(false);
        self.gate.timeline.push(Event::Released);
    }
}

#[derive(Default)]
pub struct RecordingContention(RefCell<Vec<Contention>>);

impl RecordingContention {
    pub fn recorded(&self) -> Vec<Contention> {
        self.0.borrow().clone()
    }
}

impl ContentionLog for RecordingContention {
    fn record(&self, contention: Contention) {
        self.0.borrow_mut().push(contention);
    }
}

/// Decodes `b"garbage"` as undecodable and anything else as its works.
pub struct StubOpenAlexDecoder {
    pub works: Vec<Work>,
}

impl OpenAlexDecoder for StubOpenAlexDecoder {
    fn works(&self, body: &[u8]) -> Result<Vec<Work>, DecodeFailure> {
        if body == b"garbage" {
            return Err(DecodeFailure::Undecodable);
        }
        Ok(self.works.clone())
    }

    fn work(&self, body: &[u8]) -> Result<Work, DecodeFailure> {
        self.works(body)?
            .into_iter()
            .next()
            .ok_or(DecodeFailure::Undecodable)
    }
}

pub const WITHDRAWN_RAW: &[u8] = b"raw:withdrawn";
pub const LIVE_RAW: &[u8] = b"raw:live";
pub const ERROR_FEED: &[u8] = b"error-feed";
pub const EMPTY_FEED: &[u8] = b"empty-feed";
pub const FEED: &[u8] = b"feed";

/// Decodes the named bodies above; anything else is undecodable.
pub struct StubArxivDecoder {
    pub entries: Vec<Entry>,
}

impl ArxivDecoder for StubArxivDecoder {
    fn entries(&self, body: &[u8]) -> Result<Vec<Entry>, DecodeFailure> {
        match body {
            FEED => Ok(self.entries.clone()),
            EMPTY_FEED => Ok(Vec::new()),
            ERROR_FEED => {
                Err(DecodeFailure::ErrorFeed("incorrect id format".to_owned()))
            }
            _ => Err(DecodeFailure::Undecodable),
        }
    }

    fn raw_versions(
        &self,
        body: &[u8],
    ) -> Result<Vec<RawVersion>, DecodeFailure> {
        let version = |size: &str, source_type: &str| RawVersion {
            size: size.to_owned(),
            source_type: Some(source_type.to_owned()),
        };
        match body {
            WITHDRAWN_RAW => {
                Ok(vec![version("412kb", "D"), version("0kb", "I")])
            }
            LIVE_RAW => Ok(vec![version("412kb", "D")]),
            _ => Err(DecodeFailure::Undecodable),
        }
    }
}

/// Remembers verdicts in memory and whether the gate was serving a turn
/// when each was recalled or recorded.
pub struct MemoryConfirmations<'a> {
    gate: &'a RecordingGate,
    verdicts: RefCell<Vec<(ArxivId, bool)>>,
    recalled_while_serving: RefCell<Vec<bool>>,
    recorded_while_serving: RefCell<Vec<bool>>,
}

impl<'a> MemoryConfirmations<'a> {
    pub const fn new(gate: &'a RecordingGate) -> Self {
        Self {
            gate,
            verdicts: RefCell::new(Vec::new()),
            recalled_while_serving: RefCell::new(Vec::new()),
            recorded_while_serving: RefCell::new(Vec::new()),
        }
    }

    pub fn remembering(
        gate: &'a RecordingGate,
        id: &str,
        withdrawn: bool,
    ) -> Self {
        let confirmations = Self::new(gate);
        confirmations.verdicts.borrow_mut().push((
            ArxivId::parse(id).unwrap_or_else(|error| panic!("{error}")),
            withdrawn,
        ));
        confirmations
    }

    pub fn verdicts(&self) -> Vec<(ArxivId, bool)> {
        self.verdicts.borrow().clone()
    }

    pub fn recalled_while_serving(&self) -> Vec<bool> {
        self.recalled_while_serving.borrow().clone()
    }

    pub fn recorded_while_serving(&self) -> Vec<bool> {
        self.recorded_while_serving.borrow().clone()
    }
}

impl ConfirmationCache for MemoryConfirmations<'_> {
    fn recall(&self, entry: &ArxivId) -> Option<bool> {
        self.recalled_while_serving
            .borrow_mut()
            .push(self.gate.is_serving());
        self.verdicts
            .borrow()
            .iter()
            .find(|(id, _)| id == entry)
            .map(|(_, withdrawn)| *withdrawn)
    }

    fn record(&self, entry: &ArxivId, withdrawn: bool) {
        self.recorded_while_serving
            .borrow_mut()
            .push(self.gate.is_serving());
        self.verdicts.borrow_mut().push((entry.clone(), withdrawn));
    }
}
