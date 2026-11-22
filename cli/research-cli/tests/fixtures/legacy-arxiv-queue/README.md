# Legacy arXiv queue state

The queue and pacing state an arXiv call left behind on plugin
`1.24.0-pre.105`, the last release that kept that state directly under
`<paths.tmp>/research/`. Tests copy it into a project's research scratch
directory to stand in for a project an older release has used.

- `arxiv-queue/` — the queue directory, holding one ended ticket record.
- `arxiv-pacing` — the pacing state after that call's one request.
- `ticket.txt` — the ticket the call printed.

## Capture

1. Build `accelerator-research` with the `test-loopback` feature.
2. In a scratch project, serve a mock arXiv answering every request
   `503` with `Retry-After: 30`.
3. Run `fetch arxiv lookup 2608.21129` with `ACCELERATOR_ARXIV_API_URL`
   pointed at the mock and `ACCELERATOR_RESEARCH_TEST_CALL_BUDGET_MS=35000`.
   The call sends once, cannot fit the 30 s retry in its budget, and
   prints `waiting` with its ticket.
4. Copy `arxiv-queue/` and `arxiv-pacing`, save the printed ticket, and
   delete every `.lock` file, since a held lock cannot be captured.
