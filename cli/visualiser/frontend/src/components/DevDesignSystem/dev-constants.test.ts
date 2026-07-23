import { describe, expect, it } from "vitest";
import {
  aliasSection,
  DEV_ALIAS_RE,
  DEV_CHORD,
  DEV_CHORD_HINT,
  DEV_SECTIONS,
  devAliasTarget,
} from "./dev-constants";

describe("DEV_SECTIONS", () => {
  it("lists exactly 24 sections", () => {
    expect(DEV_SECTIONS).toHaveLength(24);
  });

  it("starts at overview and ends at topbar", () => {
    expect(DEV_SECTIONS[0].id).toBe("overview");
    expect(DEV_SECTIONS[DEV_SECTIONS.length - 1].id).toBe("topbar");
  });

  // The bridge invariant: no section's canonical bare `#<id>` hash may be
  // mis-classified as an activation alias, or the scroll-spy's own writes would
  // re-trigger activation.
  it("no section id, as a bare #<id> hash, matches DEV_ALIAS_RE", () => {
    for (const s of DEV_SECTIONS) {
      expect(
        DEV_ALIAS_RE.test(`#${s.id}`),
        `section "${s.id}" must not look like an alias`,
      ).toBe(false);
    }
  });
});

describe("DEV_ALIAS_RE + aliasSection", () => {
  it("matches the activation-alias forms only", () => {
    expect(DEV_ALIAS_RE.test("#dev")).toBe(true);
    expect(DEV_ALIAS_RE.test("#dev/colors")).toBe(true);
    expect(DEV_ALIAS_RE.test("#/dev/colors")).toBe(true);
    expect(DEV_ALIAS_RE.test("#colors")).toBe(false);
    expect(DEV_ALIAS_RE.test("#overview")).toBe(false);
    expect(DEV_ALIAS_RE.test("")).toBe(false);
  });

  it("extracts the section slug from an alias", () => {
    expect(aliasSection("#dev")).toBe("");
    expect(aliasSection("#dev/colors")).toBe("colors");
    expect(aliasSection("#/dev/stagedots")).toBe("stagedots");
  });
});

describe("devAliasTarget (re-entrancy + classification)", () => {
  it("activates for an external alias, carrying the section", () => {
    expect(devAliasTarget("#dev/colors", null)).toEqual({
      activate: true,
      section: "colors",
    });
    expect(devAliasTarget("#dev", null)).toEqual({
      activate: true,
      section: undefined,
    });
  });

  it("ignores a hash the bridge itself just wrote (re-entrancy guard)", () => {
    expect(devAliasTarget("#dev/colors", "#dev/colors")).toEqual({
      activate: false,
    });
  });

  it("never activates for a bare section hash (the scroll-spy's own write)", () => {
    expect(devAliasTarget("#colors", null)).toEqual({ activate: false });
    expect(devAliasTarget("", null)).toEqual({ activate: false });
  });
});

describe("DEV_CHORD + DEV_CHORD_HINT", () => {
  it("binds the physical KeyL with shift and meta||ctrl", () => {
    expect(DEV_CHORD).toEqual({
      code: "KeyL",
      shift: true,
      meta: true,
      ctrl: true,
    });
  });

  it("resolves a non-empty platform hint mentioning shift+L", () => {
    expect(DEV_CHORD_HINT).toMatch(/L/);
    expect(DEV_CHORD_HINT.length).toBeGreaterThan(0);
  });
});
