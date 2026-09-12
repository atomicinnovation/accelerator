import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, it } from "vitest";
import { normaliseValue } from "./normalise-value";
import {
  __SETS_FOR_TEST,
  chipVariantFor,
  lifecycleToVariant,
  statusToVariant,
} from "./status-variant";
import { DOC_TYPE_KEYS } from "./types";

describe("statusToVariant", () => {
  describe("green (terminal success)", () => {
    it.each([
      "done",
      "complete",
      "accepted",
      "approved",
      "implemented",
      "final",
      "shipped",
      "resolved",
    ])("maps %s → green", (s) => {
      expect(statusToVariant(s)).toBe("green");
    });
  });

  describe("indigo (in-flight / active)", () => {
    it.each([
      "in-progress",
      "in_progress",
      "reviewed",
      "ready",
      "active",
      "proposed",
      "live",
      "monitoring",
    ])("maps %s → indigo", (s) => expect(statusToVariant(s)).toBe("indigo"));
  });

  describe("amber (needs attention)", () => {
    it.each([
      "approve-with-changes",
      "approve w/ changes",
      "Approve w/ changes",
      "review",
      "revised",
    ])("maps %s → amber", (s) => expect(statusToVariant(s)).toBe("amber"));
  });

  describe("red (blocked / terminal failure)", () => {
    it.each([
      "blocked",
      "rejected",
      "deprecated",
      "superseded",
      "abandoned",
    ])("maps %s → red", (s) => expect(statusToVariant(s)).toBe("red"));
  });

  describe("neutral (default)", () => {
    it.each(["draft", "todo", "absent"])("maps %s → neutral", (s) => {
      expect(statusToVariant(s)).toBe("neutral");
    });

    it("returns neutral for unknown strings", () => {
      expect(statusToVariant("whatever")).toBe("neutral");
    });

    it("returns neutral for ISO date strings (fallback used by LibraryTypeView)", () => {
      expect(statusToVariant("2026-04-05")).toBe("neutral");
    });

    it("returns neutral for undefined / null / empty / non-string", () => {
      expect(statusToVariant(undefined)).toBe("neutral");
      expect(statusToVariant(null)).toBe("neutral");
      expect(statusToVariant("")).toBe("neutral");
      expect(statusToVariant(42)).toBe("neutral");
      expect(statusToVariant(true)).toBe("neutral");
      expect(statusToVariant(["accepted"])).toBe("neutral");
      expect(statusToVariant({ status: "accepted" })).toBe("neutral");
    });
  });

  describe("case and separator insensitivity", () => {
    it('maps "Accepted" (capitalised) → green', () => {
      expect(statusToVariant("Accepted")).toBe("green");
    });
    it('maps "  In Progress  " → indigo', () => {
      expect(statusToVariant("  In Progress  ")).toBe("indigo");
    });
    it("treats hyphen, space, underscore, and slash equivalently", () => {
      expect(statusToVariant("in progress")).toBe("indigo");
      expect(statusToVariant("in_progress")).toBe("indigo");
      expect(statusToVariant("in-progress")).toBe("indigo");
      expect(statusToVariant("approve w/ changes")).toBe("amber");
    });
  });

  describe("internal invariants", () => {
    it("all Set keys are in normalised form", () => {
      expect(__SETS_FOR_TEST).toBeDefined();
      expect(__SETS_FOR_TEST.length).toBeGreaterThan(0);
      for (const s of __SETS_FOR_TEST) {
        expect(s.size).toBeGreaterThan(0);
        for (const k of s) {
          expect(normaliseValue(k)).toBe(k);
        }
      }
    });
  });
});

describe("lifecycleToVariant", () => {
  const cases: Array<[string, string]> = [
    ["briefed", "lifecycle-briefed"],
    ["outlined", "lifecycle-outlined"],
    ["researching", "lifecycle-researching"],
    ["synthesised", "lifecycle-synthesised"],
    ["complete", "lifecycle-complete"],
  ];

  it.each(cases)("maps %s → %s", (status, variant) => {
    expect(lifecycleToVariant(status)).toBe(variant);
  });

  it("maps each lifecycle state to a distinct variant", () => {
    const variants = new Set(
      cases.map(([status]) => lifecycleToVariant(status)),
    );
    expect(variants.size).toBe(cases.length);
  });

  it("is case- and separator-insensitive", () => {
    expect(lifecycleToVariant("  Synthesised  ")).toBe("lifecycle-synthesised");
  });

  it("falls back to neutral for an off-vocab status", () => {
    expect(lifecycleToVariant("draft")).toBe("neutral");
    expect(lifecycleToVariant(undefined)).toBe("neutral");
    expect(lifecycleToVariant("")).toBe("neutral");
  });
});

describe("chipVariantFor", () => {
  it("routes topic-research through the lifecycle ramp", () => {
    expect(chipVariantFor("topic-research", "briefed")).toBe(
      "lifecycle-briefed",
    );
    expect(chipVariantFor("topic-research", "complete")).toBe(
      "lifecycle-complete",
    );
  });

  it("routes every other doc type through the shared semantic lexicon", () => {
    for (const type of DOC_TYPE_KEYS) {
      if (type === "topic-research") continue;
      // `complete` stays green for every semantic type — the lifecycle ramp
      // must not leak into their chips.
      expect(chipVariantFor(type, "complete")).toBe("green");
      expect(chipVariantFor(type, "blocked")).toBe("red");
    }
  });

  it("leaves statusToVariant unaffected (complete stays green globally)", () => {
    expect(statusToVariant("complete")).toBe("green");
  });
});

describe("lifecycle vocabulary stays in step with the Rust schema", () => {
  // The committed fixture is emitted and guarded on the Rust side against the
  // manifest `status_vocab` in schema.rs (see the corpus schema test). Reading
  // it here — rather than hand-copying the five states — makes this a genuine
  // cross-language guard: a Rust rename regenerates the fixture, and any state
  // the TS map does not handle then falls to `neutral` and fails below.
  const vocab: string[] = JSON.parse(
    readFileSync(
      resolve(
        process.cwd(),
        "..",
        "..",
        "corpus",
        "tests",
        "fixtures",
        "topic-research-status-vocab.json",
      ),
      "utf-8",
    ),
  );

  it("maps every Rust status_vocab state to a distinct lifecycle variant", () => {
    const variants = vocab.map((state) => lifecycleToVariant(state));
    for (const [i, variant] of variants.entries()) {
      expect(
        variant,
        `state "${vocab[i]}" fell off the lifecycle ramp`,
      ).not.toBe("neutral");
    }
    expect(new Set(variants).size).toBe(vocab.length);
  });
});
