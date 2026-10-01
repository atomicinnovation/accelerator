#![allow(dead_code)]

use std::cell::Cell;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::time::Duration;
use std::time::Instant;
use std::time::SystemTime;

use research::arxiv::Entry;
use research::arxiv::RawVersion;
use research::classify::Received;
use research::classify::Response;
use research::fetch::ArxivDecoder;
use research::fetch::Attempted;
use research::fetch::Clock;
use research::fetch::ConfirmationCache;
use research::fetch::DecodeFailure;
use research::fetch::OpenAlexDecoder;
use research::fetch::PacingGate;
use research::fetch::Transport;
use research::fetch::Unavailable;
use research::openalex::Work;
use research::request::ArxivId;
use research::request::UpstreamRequest;
use research::schedule::Deadline;

pub const fn secs(seconds: u64) -> Duration {
    Duration::from_secs(seconds)
}

/// A virtual timeline: sleeping advances it instead of waiting, and every
/// wait is recorded.
pub struct RecordingClock {
    origin: Instant,
    wall_origin: SystemTime,
    elapsed: Cell<Duration>,
    slept: RefCell<Vec<Duration>>,
}

impl RecordingClock {
    pub fn new() -> Self {
        Self {
            origin: Instant::now(),
            wall_origin: SystemTime::UNIX_EPOCH + secs(1_800_000_000),
            elapsed: Cell::new(Duration::ZERO),
            slept: RefCell::new(Vec::new()),
        }
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

/// Runs every attempt at once, unless scripted to refuse a pass, and records
/// each deferral the domain asks it to persist.
#[derive(Default)]
pub struct RecordingGate {
    passes: Cell<usize>,
    refuse_from_pass: Option<(usize, Unavailable)>,
    deferrals: RefCell<Vec<Option<SystemTime>>>,
    inside: Cell<bool>,
}

impl RecordingGate {
    pub fn refusing_from(pass: usize, unavailable: Unavailable) -> Self {
        Self {
            refuse_from_pass: Some((pass, unavailable)),
            ..Self::default()
        }
    }

    pub const fn passes(&self) -> usize {
        self.passes.get()
    }

    pub fn deferrals(&self) -> Vec<Option<SystemTime>> {
        self.deferrals.borrow().clone()
    }

    pub const fn is_inside(&self) -> bool {
        self.inside.get()
    }
}

impl PacingGate for RecordingGate {
    fn paced(
        &self,
        attempt: &mut dyn FnMut() -> Attempted,
        _deadline: &Deadline,
    ) -> Result<(), Unavailable> {
        let pass = self.passes.get() + 1;
        if let Some((from, unavailable)) = &self.refuse_from_pass {
            if pass >= *from {
                return Err(*unavailable);
            }
        }
        self.passes.set(pass);
        self.inside.set(true);
        let attempted = attempt();
        self.inside.set(false);
        self.deferrals.borrow_mut().push(attempted.defer_until);
        Ok(())
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

/// Remembers verdicts in memory and whether each was recorded while the gate
/// held its pass.
pub struct MemoryConfirmations<'a> {
    gate: &'a RecordingGate,
    verdicts: RefCell<Vec<(ArxivId, bool)>>,
    recorded_inside_pass: RefCell<Vec<bool>>,
}

impl<'a> MemoryConfirmations<'a> {
    pub const fn new(gate: &'a RecordingGate) -> Self {
        Self {
            gate,
            verdicts: RefCell::new(Vec::new()),
            recorded_inside_pass: RefCell::new(Vec::new()),
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

    pub fn recorded_inside_pass(&self) -> Vec<bool> {
        self.recorded_inside_pass.borrow().clone()
    }
}

impl ConfirmationCache for MemoryConfirmations<'_> {
    fn recall(&self, entry: &ArxivId) -> Option<bool> {
        self.verdicts
            .borrow()
            .iter()
            .find(|(id, _)| id == entry)
            .map(|(_, withdrawn)| *withdrawn)
    }

    fn record(&self, entry: &ArxivId, withdrawn: bool) {
        self.recorded_inside_pass
            .borrow_mut()
            .push(self.gate.is_inside());
        self.verdicts.borrow_mut().push((entry.clone(), withdrawn));
    }
}
