---
work_item_id: "0044"
title: "Add OAuth Login"
date: "2026-04-01T10:00:00+00:00"
author: "Test Author"
type: story
status: draft
priority: high
parent: ""
tags: [auth, oauth, frontend, backend]
---

# 0044: Add OAuth Login

**Type**: Story
**Status**: Draft
**Priority**: High
**Author**: Test Author

## Summary

Add OAuth 2.0 login support so users can sign in with their Google or GitHub account instead of creating a username and password.

## Context

User research shows that 60% of prospective users abandon the signup flow. The most common feedback is that they don't want to manage another password. Supporting OAuth login with Google and GitHub is expected to significantly reduce signup friction.

## Requirements

1. Add "Sign in with Google" button to the login and signup pages.
2. Add "Sign in with GitHub" button to the login and signup pages.
3. Implement the OAuth 2.0 authorization code flow for both providers using the `passport.js` library.
4. On first OAuth login, create a new user account linked to the OAuth provider identity.
5. On subsequent OAuth logins, find and authenticate the existing linked account.
6. Store the OAuth provider name and provider user ID in the `users` table.

## Acceptance Criteria

- [ ] Given a user clicks "Sign in with Google", they are redirected to Google's OAuth consent screen.
- [ ] Given the user completes the Google OAuth flow, they are redirected back to the app and are authenticated.
- [ ] Given a new user completes the Google OAuth flow, a new user record is created in the database.
- [ ] Given an existing user who previously signed in with Google completes the Google OAuth flow, they are authenticated as the same user (no duplicate account).
- [ ] Given a user clicks "Sign in with GitHub", the same flow works for GitHub.

## Open Questions

- Should OAuth users be able to also set a password later to enable email/password login on the same account?

## Dependencies

- Blocked by:
- Blocks:

## Assumptions

- Google and GitHub OAuth apps will be registered and credentials available in the environment.
- The `users` table can be extended with `oauth_provider` and `oauth_provider_id` columns.

## Technical Notes

## Drafting Notes

- Scope limited to Google and GitHub for the initial release. Apple Sign In deferred.

## References

- Research: `meta/research/2026-03-15-oauth-providers.md`
