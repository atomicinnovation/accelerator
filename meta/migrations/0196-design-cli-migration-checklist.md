# 0196 design-CLI migration checklist

Every assertion in the deleted bash suites, mapped to the Rust test that
replaces it or recorded as a deliberate drop with a reason. This is the
evidence that nothing was lost when the five scripts and their three suites
were deleted, so it is committed rather than kept as a scratch note.

Sources:

- `scripts/test-design.sh:169-338` and `:368-430` (inline behavioural blocks)
- `skills/design/inventory-design/scripts/test-validate-source.sh` (whole file)
- `skills/design/inventory-design/scripts/test-notify-downgrade.sh` (whole file)

Rust test locations are abbreviated:

- **cli** — `cli/design-cli/tests/subcommands.rs`
- **cmd** — `cli/design-cli/src/commands.rs` unit tests
- **arg** — `cli/design-cli/src/cli.rs` unit tests
- **host** — `cli/design/src/host.rs` unit tests
- **reach** — `cli/design/src/host_reach.rs` unit tests
- **policy** — `cli/design/src/access_policy.rs` unit tests
- **cred** — `cli/design/src/credentials.rs` unit tests
- **leak** — `cli/design/src/leaked_credentials.rs` unit tests
- **cue** — `cli/design/src/cue_phrase_audit.rs` unit tests
- **down** — `cli/design/src/runtime/downgrade.rs` unit tests
- **gold** — `cli/design/tests/downgrade_goldens.rs`
- **drift** — `cli/design/tests/cue_phrase_drift.rs`
- **regex** — `cli/design-adapters/src/cue_phrases.rs` unit tests

## `test-design.sh:169-281` — `validate-source` behavioural

| Shell assertion | Replacement |
|---|---|
| script exists / is executable | **dropped** — the script is gone; the surface it asserted is now `every_migrated_script_has_a_subcommand` (cli) |
| accepts https URL | `https_and_repository_paths_are_accepted` (cli) |
| rejects `file://`, `javascript:`, `data:` | `every_refused_scheme_exits_one` (cli) |
| accepts code-repo path inside project root | `https_and_repository_paths_are_accepted` (cli) |
| rejects path with `..` escape | `a_path_escape_is_rejected_and_a_missing_directory_too` (cli) |
| accepts `http://localhost:8080`, `http://localhost/`, `http://127.0.0.1:3000`, `https://localhost:8443` | `every_loopback_form_is_accepted_with_no_flags` (cli) |
| accepts `http://LOCALHOST`, `http://localhost.`, `http://localhost:8080/path?q=1` | `every_loopback_form_is_accepted_with_no_flags` (cli) |
| rejects `http://127.0.0.2` without flag | **deliberate drop, behaviour changed** — `127.0.0.0/8` is now `Loopback` and accepted with no flag. Replacement property: `the_whole_loopback_set_needs_no_allow_internal` (cli), `the_loopback_set_covers_the_expanded_and_ranged_forms` (reach) |
| accepts `http://127.0.0.2` with `--allow-internal` | `the_whole_loopback_set_needs_no_allow_internal` (cli) — still accepted, now unconditionally |
| rejects/accepts `http://10.0.0.1`, `http://192.168.1.1` | `internal_hosts_need_allow_internal_and_are_recovered_by_it` (cli) |
| rejects `172.16.0.1` / `172.31.255.255`, stderr names `RFC1918` | `the_rfc1918_boundary_is_pinned_at_both_edges_and_just_outside` (cli); wording pinned by `the_rfc1918_rejection_names_the_reach_and_the_recovering_flag` (policy) |
| rejects `172.15.255.255` / `172.32.0.0`, stderr names `--allow-insecure-scheme` | `the_rfc1918_boundary_is_pinned_at_both_edges_and_just_outside` (cli) |
| rejects/accepts `169.254.169.254` | `internal_hosts_need_allow_internal_and_are_recovered_by_it` (cli) |
| rejects `[::1]` without flag; accepts with `--allow-internal` | **deliberate drop, behaviour changed** — `::1` is `Loopback` and accepted with no flag. Replacement: `the_whole_loopback_set_needs_no_allow_internal` (cli) |
| rejects/accepts `[fe80::1]`, `[fe80::1%eth0]` | `internal_hosts_need_allow_internal_and_are_recovered_by_it` (cli); zone-id stripping by `brackets_and_a_zone_id_are_stripped` (host) |
| rejects/accepts `[::ffff:127.0.0.1]` | **behaviour changed** — unwraps to `127.0.0.1`, so `Loopback` and accepted with no flag. Replacement: `each_headline_address_classifies_as_its_own_reach` (reach), `the_whole_loopback_set_needs_no_allow_internal` (cli) |
| rejects `[::]` without flag; accepts with `--allow-internal` | **deliberate drop, behaviour changed** — `Unspecified` is now refused under every flag. Replacement: `the_unspecified_address_is_refused_under_every_flag` (cli), `the_unspecified_address_is_rejected_under_every_flag_combination` (policy) |
| rejects `[::1]:8080` (port present) | `the_whole_loopback_set_needs_no_allow_internal` (cli) — port stripping by `brackets_and_a_zone_id_are_stripped` (host) |
| rejects/accepts `http://0.0.0.0` | **deliberate drop, behaviour changed** — as `[::]` above |
| rejects `2130706433`, `0x7f000001`, `0177.0.0.1` | `no_flag_bypasses_a_numeric_ipv4_encoding` (cli), `every_numeric_encoding_that_is_not_an_address_is_rejected` (host) |
| rejects `http://user@example.com`, `http://user:pass@127.0.0.1@evil.com` | `a_userinfo_segment_is_rejected_whatever_the_flags` (cli), `a_userinfo_segment_is_rejected` (host) |
| http-to-public gating on `--allow-insecure-scheme` alone, all four flag combinations | `http_to_a_public_host_is_gated_on_the_scheme_flag_alone` (cli) |
| stderr does not name `internal address` for a public host | `http_to_a_public_host_is_gated_on_the_scheme_flag_alone` (cli) |
| no obsolete `(not available in v1)` text | `an_internal_rejection_names_the_flag_and_the_host` (cli) |
| `http://localhost` succeeds silently | `an_accepted_location_says_nothing` (cli) |
| stderr names `--allow-internal` and the host | `an_internal_rejection_names_the_flag_and_the_host` (cli) |
| unknown flag exits 2 | `an_unknown_flag_is_a_usage_error` (cli), `an_unknown_flag_is_a_parse_failure` (arg) |

## `test-validate-source.sh` — shell-function assertions

These assert `canonicalise_host` and `classify_internal` directly. Both are
shell functions with no Rust counterpart of the same shape, and the label
vocabulary is restructured by `HostReach`, so many rows land as drops **whose
property survives elsewhere**.

| Shell assertion | Replacement |
|---|---|
| `canonicalise_host '[::1]:8080'` → `::1` | `brackets_and_a_zone_id_are_stripped` (host) |
| `canonicalise_host '[fe80::1%eth0]:443'` → `fe80::1` | `brackets_and_a_zone_id_are_stripped` (host) |
| `canonicalise_host 'LOCALHOST.'` → `localhost` | `the_host_is_lowercased`, `a_single_trailing_dot_is_stripped` (host) |
| `canonicalise_host '127.0.0.1:8080'` → `127.0.0.1` | `a_port_is_stripped` (host) |
| rejects `user:pass@example.com` | `a_userinfo_segment_is_rejected` (host) |
| rejects `2130706433` / `0x7f000001` / `0177.0.0.1` | `every_numeric_encoding_that_is_not_an_address_is_rejected` (host) |
| `is_localhost_default localhost` / `127.0.0.1` → 0 | `the_loopback_name_is_honoured_without_being_resolved`, `each_headline_address_classifies_as_its_own_reach` (reach) |
| `is_localhost_default 127.0.0.2` / `10.0.0.1` → 1 | **deliberate drop** — the two-literal carve-out no longer exists as a separate predicate. `127.0.0.2` is `Loopback` (widened, and accepted); `10.0.0.1` is `Private` and still needs the flag, pinned by `every_internal_reach_is_recovered_by_allow_internal` (policy) |
| `classify_internal 172.15.255.255` / `172.32.0.0` → public | `the_rfc1918_boundary_holds_at_both_edges_and_just_outside` (reach) |
| `classify_internal 172.16.0.0` / `172.31.255.255` → `RFC1918` | `the_rfc1918_boundary_holds_at_both_edges_and_just_outside` (reach) |
| `classify_internal 127.0.0.2` → `loopback` | `the_loopback_set_covers_the_expanded_and_ranged_forms` (reach) |
| `classify_internal ::1` / `::ffff:127.0.0.1` → `loopback` | `each_headline_address_classifies_as_its_own_reach` (reach) |
| `classify_internal 0.0.0.0` / `::` → `wildcard` | `each_headline_address_classifies_as_its_own_reach` (reach); the label survives via `every_variant_names_itself_for_the_rejection_message` (reach) |
| `classify_internal fe80::1` / `169.254.169.254` → `link-local` | `each_headline_address_classifies_as_its_own_reach` (reach) |
| `classify_internal 8.8.8.8` → public | `each_headline_address_classifies_as_its_own_reach` (reach) |

## `test-design.sh:282-315` — `resolve-auth` behavioural

| Shell assertion | Replacement |
|---|---|
| script exists / is executable | **dropped** — as above |
| header takes precedence over form vars | `a_header_takes_precedence_and_warns_about_the_ignored_form_variables` (cli), `a_header_wins_over_a_complete_form_configuration` (cred) |
| warns when form vars are ignored | same row |
| all three form vars → `form` | `all_three_form_variables_resolve_to_form` (cli, cred) |
| `USERNAME`+`PASSWORD` without `LOGIN_URL` fails fast | `a_partial_form_configuration_is_a_usage_error_naming_what_is_missing` (cli) — **exit changed 1 → 2** |
| names the missing `LOGIN_URL` var | same row, and `a_partial_form_configuration_names_every_missing_variable` (cred) |
| no env vars → `none` | `no_variables_resolve_to_none` (cli), `nothing_configured_resolves_to_none` (cred) |

## `test-design.sh:316-338` — `scrub-secrets` behavioural

| Shell assertion | Replacement |
|---|---|
| script exists / is executable | **dropped** — as above |
| clean body passes | `a_clean_body_passes_the_scrubber` (cli), `a_clean_body_names_nothing` (leak) |
| literal env-var value triggers the scrubber | `a_literal_value_is_caught_and_the_variable_named_not_the_value` (cli) |
| names the env var, not the value | same row, and `the_report_never_carries_the_value` (leak) |
| — (no shell equivalent) | **new behaviour**: `the_value_half_of_a_header_pair_is_caught` (cli), `the_value_half_of_a_header_pair_is_a_needle_of_its_own` and `the_header_name_alone_does_not_false_positive` (leak) |
| — (no shell equivalent) | **exit split**: `scrubbing_a_nonexistent_file_is_a_usage_error` (cli) — was exit 1, now exit 2 |

## `test-design.sh:359-364` — `audit-cue-phrases` call site and file

| Shell assertion | Replacement |
|---|---|
| skill body invokes `audit-cue-phrases.sh` | **rewritten and re-homed** — `scripts/test-skill-frontmatter-conformance.sh` asserts the skill invokes `accelerator design audit-cue-phrases`; `test-design.sh` kept nothing but its delegation |
| script exists / is executable | **dropped** — the script is gone. Replacement property: `every_migrated_script_has_a_subcommand` covers the surface; the two design script grants are held to their call sites by the conformance suite's "Design script grants have call sites" block |

## `test-design.sh:368-430` — `audit-cue-phrases` behavioural

| Shell assertion | Replacement |
|---|---|
| passes on a compliant fixture (all four patterns) | `all_four_cue_patterns_pass_the_audit` (cli), `each_case_insensitive_alternative_matches_in_either_case` (regex) |
| fails on a non-compliant fixture | `an_uncued_section_fails_the_audit_and_is_named` (cli) |
| fails when `implement` is followed by lowercase | `a_lowercase_implement_does_not_cue` (cli), `the_implement_pattern_stays_case_sensitive_on_its_second_word` (regex) |
| passes when an H2 is empty | `an_empty_h2_has_no_prose_to_cue` (cli), `a_whitespace_only_section_has_no_prose_to_cue` (cue) |
| cue-phrase regex file exists | `the_compiled_patterns_are_exactly_the_canonical_alternatives` (drift) — strengthened from existence to content agreement |
| — (no shell equivalent) | **exit split**: `auditing_a_nonexistent_file_is_a_usage_error` (cli) — was exit 1, now exit 2 |

## `test-notify-downgrade.sh` — whole file

| Shell assertion | Replacement |
|---|---|
| script exists / is executable | **dropped** — the script is gone |
| `notify-downgrade-messages.json` exists and is valid JSON | `the_compiled_table_still_agrees_with_the_recorded_messages` (gold) — strengthened from validity to agreement with the compiled table |
| fixtures directory exists | `every_reason_reproduces_its_golden_byte_for_byte` (gold) |
| per-reason output matches its fixture | `every_reason_reproduces_its_golden_byte_for_byte` (gold), `every_reason_prints_its_message_and_exits_zero` (cli) |
| JSON keys equal the fixture set | `no_golden_survives_without_a_reason_to_produce_it` (gold) — exhaustive by construction, since the golden test iterates the enum |
| unknown reason exits 2 | `an_unknown_or_missing_reason_is_a_usage_error` (cli), `an_unknown_reason_is_a_parse_failure` (arg) |
| missing `--reason` exits 2 | same row |
| ANSI escape stripped from output | **deliberate drop** — the filter guarded text read from a file at runtime. The table is compiled in, so the branch is unreachable by construction and testable only by mutating the shipped data. Replacement property: `every_message_is_printable_ascii_free_of_bidi_overrides` (down) |
| CR stripped from output | same row, same replacement |
| `--from` and `--to` accepted | `the_forward_compatible_flags_are_accepted` (cli), `the_forward_compatible_flags_are_accepted_and_ignored` (arg) |

## Summary of deliberate behaviour changes

| Change | Where pinned |
|---|---|
| `127.0.0.0/8` and `::1` accepted with no flag (was: `--allow-internal`) | cli, reach |
| `::ffff:127.0.0.1` unwrapped, so loopback with no flag | reach |
| `0.0.0.0` / `::` refused under every flag (was: recovered by `--allow-internal`) | cli, policy |
| `scrub-secrets` / `audit-cue-phrases` / `resolve-auth` usage error on exit 2 (was: 1) | cli |
| The value half of a `Name: value` header is a leak needle | cli, leak |
| Reserved ranges and transition encodings classified at all (was: public) | reach |

## Mutation evidence

The plan requires every checklist row for `validate-source` and `scrub-secrets`
— the SSRF boundary and the credential-scrubbing front door — to be *shown to
fail when its property is broken*, not merely to be present. Each property
below was broken in isolation and the suite re-run; the counts are the tests
that noticed.

| Property broken | Tests that caught it |
|---|---|
| Loopback carve-out removed from `access_policy` | 4 |
| Numeric encodings fall through to hostname treatment | 4 |
| IPv6 transition encodings no longer unwrapped | 5 |
| `--allow-insecure-scheme` gate removed | 4 |
| Unspecified address made recoverable by `--allow-internal` | 3 |
| Header `Name: value` pair no longer split into a value needle | 2 |
| Leak scan reports every artefact clean | 7 |
| Unreadable file returns `Failed` (exit 1) instead of `Refusal` (exit 2) | 12 |

No mutation passed unnoticed.
