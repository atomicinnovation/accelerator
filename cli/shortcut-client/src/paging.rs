//! Following v4 `next_page_url` cursors to the end of a listing.
//!
//! A listing stops for one of four reasons, and the caller needs to know
//! which: it reached its last page, it hit the page cap, the deadline passed,
//! or a page failed. A `next_page_url` that repeats would loop forever, so it
//! is a fault rather than a cut.

use std::collections::BTreeSet;

use reqwest::Url;
use serde_json::Value;

use crate::error::ClientError;
use crate::paths::Paths;
use crate::transport::Deadline;
use crate::transport::Received;
use crate::transport::Transport;

/// Why a listing stopped before its last page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cut {
    PageCap,
    Deadline,
}

impl Cut {
    #[must_use]
    pub const fn cause(self) -> &'static str {
        match self {
            Self::PageCap => "the page cap was reached",
            Self::Deadline => "the deadline passed",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pages {
    pub entities: Vec<Value>,
    pub cut: Option<Cut>,
}

#[derive(Debug)]
pub enum PageError {
    Refused(Received),
    Unusable(String),
    Client(ClientError),
    Repeated(String),
}

/// Every entity of the listing at `first`, up to `cap` pages when one is set.
///
/// # Errors
///
/// [`PageError`] for a page that failed, was not a listing, or repeated.
pub fn collect(
    transport: &Transport,
    paths: &Paths,
    first: &Url,
    cap: Option<usize>,
    deadline: &Deadline,
) -> Result<Pages, PageError> {
    let mut entities = Vec::new();
    let mut seen = BTreeSet::new();
    let mut next: Option<String> = None;
    let mut pages = 0;
    loop {
        if deadline.expired() {
            tracing::warn!(url = %first, "shortcut listing cut by the deadline");
            return Ok(Pages {
                entities,
                cut: Some(Cut::Deadline),
            });
        }
        if cap.is_some_and(|cap| pages >= cap) {
            tracing::warn!(url = %first, pages, "shortcut listing cut by the page cap");
            return Ok(Pages {
                entities,
                cut: Some(Cut::PageCap),
            });
        }
        let delivery = next.as_deref().map_or_else(
            || transport.get(first, deadline),
            |url| transport.follow(url, paths, deadline),
        );
        let received = delivery.result.map_err(PageError::Client)?;
        if !received.succeeded() {
            return Err(PageError::Refused(received));
        }
        let body = received.json().ok_or_else(|| {
            PageError::Unusable(format!("{first} answered non-JSON"))
        })?;
        let page = body.get("entities").and_then(Value::as_array).ok_or_else(
            || PageError::Unusable(format!("{first} answered no entities")),
        )?;
        entities.extend(page.iter().cloned());
        pages += 1;
        match body.get("next_page_url").and_then(Value::as_str) {
            None => {
                return Ok(Pages {
                    entities,
                    cut: None,
                })
            }
            Some(url) if !seen.insert(url.to_owned()) => {
                return Err(PageError::Repeated(url.to_owned()));
            }
            Some(url) => next = Some(url.to_owned()),
        }
    }
}
