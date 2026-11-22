---
type: "topic-research"
id: "01-what-freshness-consistency-and-cost-trade-offs-do-adaptive-p-web"
title: "What freshness, consistency and cost trade-offs do adaptive push-pull dissemination of dynamic web data (https://doi.org/10.1145/371920.372066) and exactly-once stream processing in Apache Kafka (https://doi.org/10.1145/3448016.3457556) each make, citing both papers by those URLs?"
date: "2026-11-22T01:28:08+00:00"
author: "Toby Clemson"
producer: "research-topic"
status: "complete"
kind: "finding"
round: 1
depth: 1
question: "What freshness, consistency and cost trade-offs do adaptive push-pull dissemination of dynamic web data (https://doi.org/10.1145/371920.372066) and exactly-once stream processing in Apache Kafka (https://doi.org/10.1145/3448016.3457556) each make, citing both papers by those URLs?"
source_profile: "web"
parent: "work-item:PP-891"
tags: ["research", "topic-research", "finding"]
last_updated: "2026-11-22T01:28:08+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# What freshness, consistency and cost trade-offs do adaptive push-pull dissemination of dynamic web data (https://doi.org/10.1145/371920.372066) and exactly-once stream processing in Apache Kafka (https://doi.org/10.1145/3448016.3457556) each make, citing both papers by those URLs?

## Question

What does each of two papers trade off between freshness, consistency and
cost? The first is Deolasee, Katkar, Panchbudhe, Ramamritham and Shenoy,
"Adaptive Push-Pull: Disseminating Dynamic Web Data", WWW 2001
(https://doi.org/10.1145/371920.372066). The second is Wang et al.,
"Consistency and Completeness: Rethinking Distributed Stream Processing in
Apache Kafka", SIGMOD 2021, pp. 2602–2613
(https://doi.org/10.1145/3448016.3457556).

## Findings

### Adaptive push-pull (https://doi.org/10.1145/371920.372066)

The paper is about keeping proxy or client caches of fast-changing web data
(such as stock prices) fresh enough for each user. Each client states a
**temporal coherency requirement (tcr)**: how far its cached value may drift
from the source. Freshness is measured as **fidelity**, the share of time that
requirement holds. Fidelity is the paper's single freshness and consistency
metric. The trade-offs run between fidelity, communication cost, server state
and computation cost, and resilience to failures.

- **Pure push** gives near-perfect fidelity, because the server sees every
  change and sends only changes that matter to each client's tcr. That also
  keeps message counts low. The cost falls on the server: it must keep
  per-client state (tcr, last value pushed, an open connection), which limits
  scalability. It is also fragile. If the server fails, client state is lost,
  and coherency is violated until the proxy notices and registers again.
- **Pure pull** with an adaptive time-to-refresh (TTR) is stateless and so
  recovers easily. It keeps the server light per client. The paper reports
  only about 70–80% fidelity under stringent tcrs, rising as tcrs loosen. Each
  poll costs a conditional request plus a response, and polls often outnumber
  real changes, so pull loads the network more. Heavy polling can also add
  queueing delay at the server.
- **PaP (Push-and-Pull)** runs both at once. The proxy keeps pulling with
  adaptive TTR. The server cheaply predicts the next poll and pushes
  qualifying changes only in the gap before it. A parameter ε (with TTRmax,
  TTRmin and the adaptive weight) tunes the mix: ε = 0 behaves like push, and a
  large ε behaves like pull. The paper reports fidelity above 98% under
  stringent tcrs (about 99% with ε near TTRmin) and 100% under looser ones.
  Network overhead is only slightly above push and well below pull. Because
  pulls continue, server state is "soft" and failures degrade service
  gracefully. The price is still some per-client server state and computation.
- **PoP (Push-or-Pull)** gives each connection either push or pull, depending
  on server resources. Push clients also get periodic heartbeats (TTRlimit),
  so a lost state is noticed within a bounded time. When resources run short,
  the server moves clients to pull, preferring clients that tolerate lower
  fidelity, have loose tcrs, or receive little data. It can also refuse new
  clients. The paper reports that communication overhead falls as the share
  of push rises, but pull fidelity and refused requests both suffer as pull
  load grows. With 10 ms of server cost per pull, overall fidelity dropped by
  nearly 10%. The share of push is therefore a balance, not a dial to turn up
  freely.
- **PoPoPaP** combines the two: PaP on some connections, and PoP-style
  resource allocation across all of them. The aim is to get resilience,
  coherency and scalability together.

In short, the paper pays bounded and tunable staleness and some extra
messages to buy scalability and resilience. It treats freshness as a
per-client tolerance, not a hard guarantee.

### Exactly-once in Apache Kafka (https://doi.org/10.1145/3448016.3457556)

The SIGMOD paper (Confluent, Bloomberg and Expedia authors) is about
correctness rather than bounded staleness. Its central design choice is to
split correctness into two problems and handle them with different
mechanisms. The abstract and the authors' summaries say that using one
mechanism for both causes "unnecessary performance penalties". They also say
users find it hard to make flexible trade-offs among "correctness,
performance, and cost".

- **Consistency** means no lost or duplicated effects after failure
  (exactly-once). Kafka Streams treats processing as read-process-write
  cycles. State updates and outputs are recorded as appends to Kafka's
  persistent log, which is both the storage layer and the channel between
  processors. Exactly-once comes from idempotent and transactional writes,
  with a two-phase commit over each read-process-write cycle. The cost is
  framed as "a modest cost to persist streaming data". In return there is no
  coordination inside processors, which the authors say adds end-to-end
  latency in frameworks that use one unified mechanism.
- **Completeness** means handling out-of-order data correctly. It uses
  **revision-based speculative processing**: results are emitted as soon as
  possible and revised as late records arrive, and only operators that are
  sensitive to ordering do this. This is the paper's freshness trade-off.
  Downstream consumers get low latency, but they see provisional results that
  may later change, instead of waiting for input to be complete.
- The paper says this split lets users "decouple the fundamental trade-off
  decisions between latency, throughput, and correctness guarantees", and it
  calls the resulting trade-offs "low-overhead" in large production
  deployments. The paper's own measurements could not be retrieved, so they
  are not reported here.

The earlier Confluent engineering post on exactly-once in Kafka Streams, by
the same lead author (tier-2, not the paper), gives the concrete cost lever.
The commit interval (`commit.interval.ms`) sets the transaction size. Shorter
intervals mean smaller transactions and lower end-to-end latency, but less
throughput. Read-committed consumers see a transaction's records only after
it commits, so the commit interval also limits how fresh downstream data can
be. The figures reported there:

| Setting | Reported throughput cost |
|---|---|
| Transactional producer, 1 KB messages, 100 ms transactions | about 3% below an in-order at-least-once producer |
| Same producer, compared with the default producer | about 20% |
| Kafka Streams, 100 ms commit interval | 15–30%, depending on message size |
| Kafka Streams, 30 s commit interval, messages of 1 KB or more | no overhead |
| Idempotence alone | negligible |

### Contrast

The push-pull paper treats consistency as **bounded divergence** for each
client (tcr and fidelity). It trades freshness against network messages,
server state and failure resilience, and tunes the balance with ε, TTR
bounds and the share of push clients. The Kafka paper treats consistency as
**exactly-once effects**, which it never relaxes. It trades freshness in two
other ways: through commit batching (latency against throughput) and through
speculative emission (early, revisable results against waiting for
completeness). Its cost is persisting everything to the log and running
transactions. In both papers the cost falls on whoever must track state.
That is the server's per-client push state in the first paper and the
transaction and changelog log in the second. Both expose knobs rather than
fixing one point on the trade-off curve.

## Sources

- [Adaptive Push-Pull: Disseminating Dynamic Web Data (DOI)](https://doi.org/10.1145/371920.372066) — tier-1 — doi.org / ACM, WWW 2001 proceedings
- [Adaptive Push-Pull: Disseminating Dynamic Web Data (full text, WWW10 archive)](https://archives.iw3c2.org/www10/cdrom/papers/269/index.html) — tier-1 — archives.iw3c2.org (IW3C2, WWW conference organiser)
  - accessed: 2026-11-22
- [Consistency and Completeness: Rethinking Distributed Stream Processing in Apache Kafka (DOI)](https://doi.org/10.1145/3448016.3457556) — tier-1 — doi.org / ACM, SIGMOD 2021 proceedings
- [Crossref record for 10.1145/3448016.3457556](https://api.crossref.org/works/10.1145%2F3448016.3457556) — tier-1 — api.crossref.org
- [Consistency and Completeness: Rethinking Distributed Stream Processing in Apache Kafka (blog)](https://www.confluent.io/blog/rethinking-distributed-stream-processing-in-kafka/) — tier-2 — confluent.io
  - published: 2021-06-18
  - updated: 2022-09-28
  - accessed: 2026-11-22
- [Consistency and Completeness [New White Paper] (abstract)](https://forum.confluent.io/t/consistency-and-completeness-rethinking-distributed-stream-processing-in-apache-kafka-new-white-paper/1827) — tier-2 — forum.confluent.io
  - published: 2021-05-19
  - accessed: 2026-11-22
- [Enabling Exactly-Once in Kafka Streams](https://www.confluent.io/blog/enabling-exactly-once-kafka-streams/) — tier-2 — confluent.io
  - published: 2017-06-30
  - updated: 2025-04-01
  - accessed: 2026-11-22
