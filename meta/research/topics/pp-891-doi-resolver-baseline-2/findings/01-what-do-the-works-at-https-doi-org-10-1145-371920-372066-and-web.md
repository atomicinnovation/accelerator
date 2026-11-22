---
type: "topic-research"
id: "01-what-do-the-works-at-https-doi-org-10-1145-371920-372066-and-web"
title: "What do the works at https://doi.org/10.1145/371920.372066 and https://doi.org/10.1145/3448016.3457556 each contribute, and what evidence does each offer, citing both by those URLs?"
date: "2026-11-22T09:15:00+00:00"
author: "Toby Clemson"
producer: "research-topic"
status: "complete"
kind: "finding"
round: 1
depth: 1
question: "What do the works at https://doi.org/10.1145/371920.372066 and https://doi.org/10.1145/3448016.3457556 each contribute, and what evidence does each offer, citing both by those URLs?"
source_profile: "web"
parent: "work-item:PP-891"
tags: ["research", "topic-research", "finding"]
last_updated: "2026-11-22T09:15:00+00:00"
last_updated_by: "Toby Clemson"
schema_version: 1
---

# What do the works at https://doi.org/10.1145/371920.372066 and https://doi.org/10.1145/3448016.3457556 each contribute, and what evidence does each offer, citing both by those URLs?

## Question

What do the works at https://doi.org/10.1145/371920.372066 and
https://doi.org/10.1145/3448016.3457556 each contribute, and what evidence
does each offer, citing both by those URLs?

## Findings

Both DOIs resolve, via a `302` from doi.org, to ACM Digital Library landing
pages (`dl.acm.org/doi/...`). Those pages returned HTTP 403 to automated
fetches, so the bibliographic details below come from each DOI's Crossref
record. The content and evidence come from the open-access proceedings
archive for the first work and from the authors' and publisher's own
summaries for the second.

### https://doi.org/10.1145/371920.372066: Adaptive push-pull (WWW10, 2001)

**Identity.** According to Crossref, this DOI is "Adaptive push-pull:
disseminating dynamic web data" by Pavan Deolasee, Amol Katkar, Ankur
Panchbudhe, Krithi Ramamritham and Prashant Shenoy. It appeared in the
*Proceedings of the 10th International Conference on World Wide Web*
(WWW10, Hong Kong, 2001), pp. 265–274, published by ACM.

**Contribution.** The paper looks at how to keep proxy copies of
fast-changing web data, such as stock prices and sports scores, coherent
with the origin server. Coherency is defined in value terms: the user's
copy may differ from the server's by at most a user-set tolerance *c*.
*Fidelity* is the fraction of time that bound holds. The paper sets out the
trade-offs between the two standard approaches:

- **Push** gives high fidelity with few messages. In exchange, the server
  must keep per-client state and check every change against every client's
  tolerance, and it is fragile when the server fails.
- **Pull** (HTTP polling with an adaptive time-to-refresh, TTR) is stateless
  and resilient. Its fidelity is lower for volatile data or tight
  tolerances, and its message count grows quickly.

The paper proposes two adaptive hybrids:

- **PaP (Push-and-Pull).** The proxy pulls using adaptive TTRs. The server
  predicts the proxy's next poll and pushes any change the proxy would
  otherwise miss before then. A tunable parameter ε controls the mix:
  ε = 0 behaves as pure push, and a large ε behaves as pure pull. The
  server keeps only soft state, so the system degrades gracefully when
  that state is lost.
- **PoP (Push-or-Pull).** The server assigns each connection to push or
  pull according to its resources. Push is the default. Under load, the
  server moves to pull those clients that tolerate lower fidelity or have
  looser tolerances. Heartbeats let push clients detect a server failure
  within a bounded time.

A further combination, PoPoPaP, uses PaP in place of plain push inside PoP.

**Evidence.** The evaluation is empirical, using a prototype server and
proxy that replayed traces on a local intranet:

- **Data.** The traces are real stock quotes from Yahoo Finance for eight
  companies, sampled 2–3 times per second in June 2000. Each two-hour
  trace holds about 15,000 values.
- **Settings.** Pull used adaptive TTR with TTR_min = 1 s and TTR_max of
  10, 30 or 60 s. Tolerances ranged from $0.05 to $0.40.
- **Push vs. pull.** Pure push reached 100% fidelity. Pull reached about
  70–80% at stringent tolerances, at a much higher message cost.
- **PaP.** Fidelity exceeded 98% at stringent tolerances (about 99% at
  ε = 1) and reached 100% at lax ones. Its message count was far below
  pull's and only slightly above push's.
- **PoP.** Tests ran with four client machines of 56 users each. Shifting
  connections toward push cut communication overhead. When each pull cost
  10 ms of server computation, pull requests were denied under load and
  overall fidelity fell by nearly 10%.

The authors conclude that no single dissemination method suits every
combination of data and user. Adapting the push/pull mix to coherency
requirements, data dynamics and load gives better resilience than pure push
and better scalability and fidelity than pure pull. Search results also
point to an extended journal version in *IEEE Transactions on Computers*
(2002). That pointer was not followed up here.

### https://doi.org/10.1145/3448016.3457556: Consistency and completeness in Kafka Streams (SIGMOD 2021)

**Identity.** According to Crossref, this DOI is "Consistency and
Completeness: Rethinking Distributed Stream Processing in Apache Kafka" by
Guozhang Wang, Lei Chen, Ayusman Dikshit, Jason Gustafson, Boyang Chen,
Matthias J. Sax, John Roesler, Sophie Blee-Goldman, Bruno Cadonna, Apurva
Mehta, Varun Madan and Jun Rao. It appeared in the *Proceedings of the 2021
International Conference on Management of Data* (SIGMOD '21), pp.
2602–2613. It is a joint industrial paper from Confluent, Bloomberg and
Expedia.

**Contribution.** The paper argues that stream processing can give strong
correctness guarantees rather than being treated as approximate or lossy.
It splits correctness into two properties:

- **Consistency.** After a failure, recovery produces no duplicates and no
  lost results, which is exactly-once semantics.
- **Completeness.** Final results are correct even when records arrive out
  of order, because event time and processing time can diverge.

The central design choice is to use Kafka's replicated, append-only log as
both the storage layer and the communication layer between processors.
Kafka Streams models processing as read-process-write cycles. Committing
input offsets, updating state and writing output are all expressed as
appends to that log:

- **Consistency mechanism.** Idempotent and transactional writes, under a
  two-phase-commit-style protocol, make those appends exactly-once. In the
  authors' account this needs only a single write to the log, where a
  conventional two-phase commit writes both to state and to a log.
- **Completeness mechanism.** Revision-based speculative processing emits
  results early and revises them when late data arrives. It is applied
  only to the operators that are sensitive to ordering.

Keeping the two mechanisms separate is meant to let users trade off
latency, throughput and correctness independently, instead of paying the
end-to-end latency of one unified approach such as global checkpoints or
watermarks.

**Evidence.** The paper's evidence is industrial rather than a controlled
benchmark study. Its abstract says it "demonstrates how Kafka Streams behaves
in practice with large-scale deployments and performance insights." The
deployments named are at Bloomberg and Expedia. In the publisher's podcast,
the lead author describes the paper as a summary of lessons from years of
running Kafka Streams and ksqlDB in production, building on prior work
rather than claiming wholly new ideas. The latency advantage over
checkpoint- and watermark-based designs is presented there as an expected
benefit of the design, not as a measured result. None of the accessible
sources report the paper's quantitative results, such as exactly-once
overhead, throughput or latency figures. The full text sits behind the
ACM landing page, which refused automated access, and behind Confluent's
gated white-paper form. Its specific numbers therefore remain unverified
here.

### Comparison

Both works study the trade-off between freshness or correctness and cost
when propagating changing data. They frame it differently:

- **10.1145/371920.372066** treats coherency as a tunable, value-bounded
  quality (fidelity within tolerance *c*). It adapts the delivery mechanism,
  push versus pull, to balance fidelity against message, compute and state
  overhead. Its evidence is a controlled, trace-driven prototype
  evaluation with reported fidelity and message-count results.
- **10.1145/3448016.3457556** treats correctness as two hard guarantees
  (exactly-once consistency and out-of-order completeness), built on a
  persistent transactional log. Its evidence rests mainly on production
  deployment experience rather than on figures that could be checked
  from the accessible sources.

## Sources

- [Adaptive push-pull: disseminating dynamic web data (DOI 10.1145/371920.372066)](https://doi.org/10.1145/371920.372066) — tier-1 — doi.org → dl.acm.org (ACM Digital Library; WWW10 proceedings)
- [Adaptive push-pull: disseminating dynamic web data — Crossref record](https://api.crossref.org/works/10.1145/371920.372066) — tier-1 — api.crossref.org (Crossref DOI registration agency)
- [Adaptive Push-Pull: Disseminating Dynamic Web Data (WWW10 proceedings archive)](https://archives.iw3c2.org/www10/cdrom/papers/269/index.html) — tier-1 — archives.iw3c2.org (IW3C2, WWW conference steering body)
  - accessed: 2026-11-22
- [Consistency and Completeness: Rethinking Distributed Stream Processing in Apache Kafka (DOI 10.1145/3448016.3457556)](https://doi.org/10.1145/3448016.3457556) — tier-1 — doi.org → dl.acm.org (ACM Digital Library; SIGMOD '21 proceedings)
- [Consistency and Completeness: Rethinking Distributed Stream Processing in Apache Kafka — Crossref record](https://api.crossref.org/works/10.1145/3448016.3457556) — tier-1 — api.crossref.org (Crossref DOI registration agency)
- [Consistency and Completeness: Rethinking Distributed Stream Processing in Apache Kafka (Confluent blog)](https://www.confluent.io/blog/rethinking-distributed-stream-processing-in-kafka/) — tier-2 — confluent.io (vendor engineering blog, by the paper's lead author)
  - published: 2021-06-18
  - updated: 2022-09-28
  - accessed: 2026-11-22
- [Consistency and Completeness: Rethinking Distributed Stream Processing in Apache Kafka [New White Paper]](https://forum.confluent.io/t/consistency-and-completeness-rethinking-distributed-stream-processing-in-apache-kafka-new-white-paper/1827) — tier-3 — forum.confluent.io (community forum)
  - published: 2021-05-19
  - accessed: 2026-11-22
- [Consistent, Complete Distributed Stream Processing ft. Guozhang Wang (Streaming Audio podcast)](https://developer.confluent.io/podcast/consistent-complete-distributed-stream-processing-ft-guozhang-wang) — tier-2 — developer.confluent.io (vendor developer podcast)
  - accessed: 2026-11-22
