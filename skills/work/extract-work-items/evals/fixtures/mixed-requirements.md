# Q2 Product Requirements

## Bug: Dashboard Charts Show Stale Data After Navigation

Users report that after navigating away from the analytics dashboard and returning,
the charts display yesterday's data rather than updating to today's figures.

**Observed behaviour**: Charts show previous day's metrics even after a page refresh.
**Expected behaviour**: Charts should load with the latest available data on every visit.
**Reproduction steps**:
1. Visit the analytics dashboard and note the current day's figures
2. Navigate to any other page via the sidebar
3. Return to the dashboard via the sidebar link
4. Observe: yesterday's data is displayed instead of today's

**Environment**: Chrome 121, Firefox 122, Safari 17.2 — all affected on production.

---

## Feature: CSV Export for Monthly Revenue Report

Finance team members need to download the monthly revenue report as a CSV file
for import into Excel. Currently they screenshot or manually transcribe data.

As a finance team member, I want to export the monthly revenue report as a CSV
so that I can analyse trends in spreadsheet software without manual data entry.

The export should cover all columns shown in the current table view and respect
the active date range filter.

---

## Research: Evaluate Real-Time Search Options for Documentation Site

We receive frequent support requests that could be self-served if users could
search our docs. Before committing to an implementation, we need to understand
the landscape.

Time-box: 3 days
Research questions:
- Which hosted search services (Algolia, Typesense, Pagefind) best fit a static
  site with approximately 2,000 pages?
- What is the implementation cost for each option?
- What are the indexing latency characteristics of each?
Exit criterion: A written comparison with a clear recommendation and estimated
implementation effort.

---

## Initiative: Modernise User Authentication System

Our current auth system uses basic session cookies with no MFA support, and
several enterprise clients have flagged this as a blocker for procurement.

This is a major cross-functional initiative spanning multiple quarters:
- Replace session cookies with JWT-based authentication
- Add TOTP and hardware key MFA options
- Implement SSO via SAML 2.0 for enterprise clients
- Migrate existing sessions without disruption
- Harden refresh token rotation handling

Each workstream will decompose into multiple user stories and tasks.
