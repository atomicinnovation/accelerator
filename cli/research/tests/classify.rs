use std::time::Duration;

use research::classify::arxiv;
use research::classify::openalex;
use research::classify::ClientRejection;
use research::classify::FetchError;
use research::classify::Reason;
use research::classify::Received;
use research::classify::Response;
use research::classify::Verdict;
use research::request::Family;
use research::request::KeySource;
use research::request::Verb;

fn status(code: u16) -> Response {
    Response::Received(Received::status(code))
}

fn rate_limited(remaining: Option<u64>, required: Option<u64>) -> Response {
    Response::Received(Received {
        rate_limit_remaining: remaining,
        rate_limit_credits_required: required,
        ..Received::status(429)
    })
}

fn keyed() -> KeySource {
    KeySource::new("ACCELERATOR_OPENALEX_API_KEY")
}

const fn retry(reason: Reason) -> Verdict {
    Verdict::Retry {
        reason,
        retry_after: None,
    }
}

const fn client_error(family: Family, rejection: ClientRejection) -> Verdict {
    Verdict::Fail(FetchError::ClientError { family, rejection })
}

#[test]
fn a_success_delivers_its_body_from_either_source() {
    let delivered = Response::Received(Received {
        body: b"{}".to_vec(),
        ..Received::status(200)
    });
    assert_eq!(
        openalex(delivered.clone(), Verb::Search, None),
        Verdict::Deliver(b"{}".to_vec())
    );
    assert_eq!(arxiv(delivered), Verdict::Deliver(b"{}".to_vec()));
}

#[test]
fn an_unfollowed_redirect_fails_from_either_source() {
    assert_eq!(
        openalex(status(301), Verb::Lookup, None),
        client_error(Family::OpenAlex, ClientRejection::Redirect(301))
    );
    assert_eq!(
        arxiv(status(302)),
        client_error(Family::Arxiv, ClientRejection::Redirect(302))
    );
}

#[test]
fn an_openalex_lookup_miss_is_empty() {
    assert_eq!(openalex(status(404), Verb::Lookup, None), Verdict::Empty);
}

#[test]
fn an_openalex_search_not_found_is_a_client_error() {
    assert_eq!(
        openalex(status(404), Verb::Search, None),
        client_error(Family::OpenAlex, ClientRejection::Status(404))
    );
}

#[test]
fn an_openalex_conflict_is_budget_exhaustion() {
    assert_eq!(
        openalex(status(409), Verb::Search, None),
        Verdict::Unavailable(Reason::BudgetExhausted)
    );
}

#[test]
fn an_openalex_429_short_of_the_credits_required_is_budget_exhaustion() {
    assert_eq!(
        openalex(rate_limited(Some(5), Some(10)), Verb::Search, None),
        Verdict::Unavailable(Reason::BudgetExhausted)
    );
    assert_eq!(
        openalex(rate_limited(Some(0), None), Verb::Search, None),
        Verdict::Unavailable(Reason::BudgetExhausted)
    );
}

#[test]
fn any_other_openalex_429_is_retried_as_rate_limiting() {
    for (remaining, required) in [
        (Some(10), Some(10)),
        (Some(3), None),
        (None, None),
        (None, Some(1)),
    ] {
        assert_eq!(
            openalex(rate_limited(remaining, required), Verb::Search, None),
            retry(Reason::RateLimited),
            "remaining {remaining:?}, required {required:?}"
        );
    }
}

#[test]
fn a_retry_carries_the_upstream_hint() {
    let hinted = Response::Received(Received {
        retry_after: Some(Duration::from_secs(5)),
        ..Received::status(429)
    });
    assert_eq!(
        openalex(hinted.clone(), Verb::Search, None),
        Verdict::Retry {
            reason: Reason::RateLimited,
            retry_after: Some(Duration::from_secs(5)),
        }
    );
    assert_eq!(
        arxiv(hinted),
        Verdict::Retry {
            reason: Reason::RateLimited,
            retry_after: Some(Duration::from_secs(5)),
        }
    );
}

#[test]
fn an_openalex_auth_refusal_names_the_key_that_was_rejected() {
    for code in [401, 403] {
        assert_eq!(
            openalex(status(code), Verb::Search, Some(&keyed())),
            Verdict::Fail(FetchError::KeyRejected(keyed()))
        );
    }
}

#[test]
fn a_keyless_openalex_auth_refusal_is_unauthenticated() {
    for code in [401, 403] {
        assert_eq!(
            openalex(status(code), Verb::Search, None),
            Verdict::Fail(FetchError::Unauthenticated)
        );
    }
}

#[test]
fn other_openalex_client_errors_fail() {
    for code in [400, 406, 410, 422] {
        assert_eq!(
            openalex(status(code), Verb::Search, Some(&keyed())),
            client_error(Family::OpenAlex, ClientRejection::Status(code))
        );
    }
}

#[test]
fn arxiv_throttling_is_retried_as_rate_limiting() {
    for code in [403, 406, 429] {
        assert_eq!(arxiv(status(code)), retry(Reason::RateLimited), "{code}");
    }
}

#[test]
fn other_arxiv_client_errors_fail() {
    for code in [400, 401, 404] {
        assert_eq!(
            arxiv(status(code)),
            client_error(Family::Arxiv, ClientRejection::Status(code))
        );
    }
}

#[test]
fn server_errors_and_transport_failures_are_retried_as_upstream_errors() {
    for response in [
        status(500),
        status(503),
        Response::ConnectionFailed,
        Response::TimedOut,
    ] {
        assert_eq!(
            openalex(response.clone(), Verb::Search, None),
            retry(Reason::UpstreamError)
        );
        assert_eq!(arxiv(response), retry(Reason::UpstreamError));
    }
}

#[test]
fn a_reason_renders_as_its_output_code() {
    assert_eq!(Reason::RateLimited.code(), "rate_limited");
    assert_eq!(Reason::BudgetExhausted.code(), "budget_exhausted");
    assert_eq!(Reason::UpstreamError.code(), "upstream_error");
}

#[test]
fn each_failure_renders_its_coded_message() {
    let cases = [
        (
            FetchError::KeyRejected(KeySource::new("openalex.api_key")),
            "E_OPENALEX_KEY_REJECTED: the OpenAlex API key from \
             openalex.api_key was rejected",
        ),
        (
            FetchError::Unauthenticated,
            "E_OPENALEX_UNAUTHENTICATED: OpenAlex refused a keyless request \
             — configure openalex.api_key",
        ),
        (
            FetchError::UndecodableResponse(Family::Arxiv),
            "E_RESEARCH_UNDECODABLE: arXiv returned a response that could \
             not be read",
        ),
        (
            FetchError::ClientError {
                family: Family::OpenAlex,
                rejection: ClientRejection::Status(400),
            },
            "E_RESEARCH_CLIENT_ERROR: OpenAlex rejected the request with \
             HTTP 400",
        ),
        (
            FetchError::ClientError {
                family: Family::OpenAlex,
                rejection: ClientRejection::Redirect(301),
            },
            "E_RESEARCH_CLIENT_ERROR: OpenAlex redirected the request with \
             HTTP 301 to a location that is not followed",
        ),
        (
            FetchError::ClientError {
                family: Family::Arxiv,
                rejection: ClientRejection::ErrorFeed(
                    "incorrect id format".to_owned(),
                ),
            },
            "E_RESEARCH_CLIENT_ERROR: arXiv rejected the request: incorrect \
             id format",
        ),
    ];
    for (error, message) in cases {
        assert_eq!(error.to_string(), message);
    }
}
