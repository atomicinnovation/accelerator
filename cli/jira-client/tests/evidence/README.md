# Contract-run evidence

This directory holds `contract-run.txt`: the reduced record of a live-tenant
`RemoteTracker` contract run against Jira, committed as the live assurance
beside the offline conformance suite (`contract_offline.rs`), which is the
enforcing gate.

The committed `contract-run.txt` was produced against a credentialed tenant;
regenerate it with:

```bash
ACCELERATOR_TRACKER_CONTRACT=1 \
ACCELERATOR_TRACKER_CONTRACT_EVIDENCE="$PWD/cli/jira-client/tests/evidence/contract-run.txt" \
ACCELERATOR_TRACKER_CONTRACT_DATE=<YYYY-MM-DD> \
mise run test:integration:tracker-contract
```

alongside the `ACCELERATOR_JIRA_*` variables the harness names when a tenant is
absent. The harness emits only test name, outcome, count and duration — no
payloads. `evidence_hygiene.rs` refuses a committed file carrying anything else.
