# YAML Block-Sequence Array Parsing

## Problem

The config parser (Plan 1's `config-read-value.sh`) only supports inline YAML
arrays:

```yaml
disabled_lenses: [portability, compatibility]
```

It does **not** support block-sequence arrays:

```yaml
disabled_lenses:
  - portability
  - compatibility
```

Block-sequence syntax is natural YAML and users will reach for it. When they do,
the parser silently returns nothing — the key appears to have no value. No
warning is emitted. The user's configuration is quietly ignored.

This affects all array-valued config keys: `disabled_lenses`, `core_lenses`,
and any future array keys.

## Current Mitigation

- The configure skill documents inline-only format in examples
- The configure skill's `create` action generates inline format

This is insufficient long-term — users edit config files by hand.

## Recommended Fix Options (in order of preference)

### 1. Extend the awk parser to handle `- item` lines

Add ~20-30 lines to the awk parser in `config-read-value.sh` to detect
`- item` lines under a key and return them as `[a, b, c]` inline format.
Keeps the downstream `config_parse_array` function unchanged.

Contained to `scripts/config-read-value.sh` in Plan 1's codebase.

### 2. Vendor a lightweight shell YAML parser

Best candidates (MIT licensed, single-file):
- **azohra/yaml.sh** (~194 lines) — handles block arrays, partial inline
  array support, would need a small patch for `[a, b, c]`
- **jasperes/bash-yaml** (~89 lines) — small, would need array additions
- **sopos/yash** (~844 lines) — full feature coverage but large

### 3. Require `yq` as a dependency

The plugin already requires `jq`. Requiring `yq` (mikefarah/yq, MIT, single
static Go binary) would give full YAML spec compliance permanently. Same
install pattern as `jq` (`brew install yq`).

### 4. Detect-and-warn

In array-consuming scripts (e.g., `config-read-review.sh`), check whether a
key returns empty but the raw frontmatter contains that key followed by
`- ` lines. Emit a warning to stderr pointing the user to inline syntax.

This doesn't fix the problem but makes it visible.

## References

- Plan 1 parser constraints: `meta/plans/2026-03-23-config-infrastructure.md`,
  lines 63-81
- Plan 3 array usage: `meta/plans/2026-03-23-review-system-customisation.md`,
  lines 143-145, 165-198
- Research on shell YAML parsers conducted 2026-03-24 during stress-test of
  Plan 3
