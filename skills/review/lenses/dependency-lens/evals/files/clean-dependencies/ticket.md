---
title: "Add OAuth 2.0 login via Google"
type: story
status: ready
priority: medium
---

# Add OAuth 2.0 Login via Google

## Summary

Allow users to sign in with their Google account using OAuth 2.0, as an
alternative to email-and-password authentication.

## Context

A significant portion of sign-up drop-off occurs at the password-creation
step. Analytics show that 34% of users who reach the sign-up form abandon
it before creating a password. Social sign-in is expected to reduce this
friction. Google is the priority provider because it covers the largest
share of our user base's email domains.

## Requirements

1. Add a "Sign in with Google" button to the login and sign-up pages.
2. Implement the OAuth 2.0 authorisation code flow: redirect to Google's
   authorisation endpoint, exchange the authorisation code for tokens, and
   retrieve the user's profile (email, display name).
3. On successful OAuth callback, create a new user record if the email is
   not already registered, or link the Google account to an existing user
   record if the email matches.
4. Issue a session token (existing session infrastructure) after
   successful OAuth authentication.

## Acceptance Criteria

- A user can click "Sign in with Google" on the login page and be
  authenticated without creating a password.
- A user who previously registered with email-and-password can sign in
  via Google using the same email address; the accounts are merged, not
  duplicated.
- A failed OAuth callback (user denies permission or Google returns an
  error) displays an appropriate error message and returns the user to the
  login page.
- The Google client ID and client secret are read from the secrets manager,
  not hardcoded.

## Dependencies

- **Blocked by**: #0987 "Rotate shared-secrets service to v2 API" — the
  new OAuth credentials will be stored in the v2 secrets format; the
  current service does not support v2 secrets reads.
- **Blocked by**: #1005 "Add Google OAuth application in GCP console" —
  the client ID and client secret required for the OAuth flow must exist
  before this story can be tested end-to-end.
- **Blocks**: #1089 "Add GitHub OAuth login" — that story reuses the OAuth
  callback infrastructure introduced here.
- **Blocks**: #1090 "Add SAML SSO for Enterprise plan" — the session-
  issuance path introduced here is a prerequisite for the SSO story.
- **External**: Google OAuth API (accounts.google.com/o/oauth2) — if
  Google's authorisation endpoint is unavailable, the login flow is
  degraded to email-and-password only. Google publishes a 99.9% monthly
  SLA for OAuth endpoints.

## Assumptions

- The existing session infrastructure supports issuing a session after an
  OAuth callback without modification.
- Users who sign in via Google and later want a password can use the
  "forgot password" flow, which sends a reset link to their Google email.
