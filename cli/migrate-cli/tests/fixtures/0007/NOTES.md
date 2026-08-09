This fixture captures migration 0007 against a small, fully base-field-valid two-work-item corpus with a `## References` section pointing at another real work item.

It does **not** exercise the `MIGRATION STALLED` / interactive-prompt path: both an existing-target reference (`work-item:0002`, resolvable) and a non-existent-target reference (`work-item:0099`) were tried empirically and both apply mechanically with no decision required — the latter is silently dropped (a reference to an unresolvable target is dropped, not treated as ambiguous), not treated as ambiguous.

Triggering the genuinely `ambiguous`-band case that requires a human decision needs a reference shape that is structurally ambiguous by construction (per `corpus::linkage::classify_band`), not merely an unresolved target — that case is exercised separately, in `migration_0007.rs`'s own ambiguous-band stall test and in `list_and_decisions_file.rs`.
