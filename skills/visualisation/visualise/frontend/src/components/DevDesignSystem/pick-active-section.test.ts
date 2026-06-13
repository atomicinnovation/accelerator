import { describe, expect, it } from "vitest";
import { pickActiveSection, type SectionTop } from "./pick-active-section";

const BAND = 80;

describe("pickActiveSection", () => {
  it("returns null for no sections", () => {
    expect(pickActiveSection([], BAND)).toBeNull();
  });

  it("scrolled above all → the first section (never cleared)", () => {
    const s: SectionTop[] = [
      { id: "a", top: 120 },
      { id: "b", top: 300 },
      { id: "c", top: 500 },
    ];
    expect(pickActiveSection(s, BAND)).toBe("a");
  });

  it("scrolled below all → the last section", () => {
    const s: SectionTop[] = [
      { id: "a", top: -500 },
      { id: "b", top: -300 },
      { id: "c", top: -100 },
    ];
    expect(pickActiveSection(s, BAND)).toBe("c");
  });

  it("picks the last section whose top is at/above the band", () => {
    const s: SectionTop[] = [
      { id: "a", top: -200 },
      { id: "b", top: 50 },
      { id: "c", top: 400 },
    ];
    expect(pickActiveSection(s, BAND)).toBe("b");
  });

  it("keeps the upper section active in the gap between two short sections", () => {
    // a ends above the band; b starts below it (band 80 sits in the a→b gap).
    const s: SectionTop[] = [
      { id: "a", top: -100 },
      { id: "b", top: 140 },
      { id: "c", top: 360 },
    ];
    expect(pickActiveSection(s, BAND)).toBe("a");
  });

  it("keeps a tall section spanning the whole band active (the never-pinned fix)", () => {
    // colours is tall: its top is far above the band, the next short section's
    // top is still below the band → colours stays active, not pinned-elsewhere.
    const s: SectionTop[] = [
      { id: "overview", top: -900 },
      { id: "colours", top: -400 },
      { id: "type", top: 300 },
    ];
    expect(pickActiveSection(s, BAND)).toBe("colours");
  });

  it("treats a top exactly at the band as at/above (inclusive)", () => {
    const s: SectionTop[] = [
      { id: "a", top: -50 },
      { id: "b", top: 80 },
      { id: "c", top: 200 },
    ];
    expect(pickActiveSection(s, BAND)).toBe("b");
  });
});
