import { describe, expect, test } from "vitest";
import type { DocTypeKey } from "../../api/types";
import { clusterViaLabel } from "./cluster-via-label";

const cases: Array<
  [DocTypeKey, { entryKey: string | null; clusterKey: string | null }, string]
> = [
  [
    "plans",
    { entryKey: "0040", clusterKey: "0040" },
    "clustered via: parent → work-item:0040",
  ],
  [
    "research",
    { entryKey: "0040", clusterKey: "0040" },
    "clustered via: parent → work-item:0040",
  ],
  [
    "work-items",
    { entryKey: "0040", clusterKey: "0040" },
    "clustered via: parent → work-item:0040",
  ],
  [
    "pr-descriptions",
    { entryKey: "0040", clusterKey: "0040" },
    "clustered via: parent → work-item:0040",
  ],
  [
    "validations",
    { entryKey: "0040", clusterKey: "0040" },
    "clustered via: target → plan → parent",
  ],
  [
    "plan-reviews",
    { entryKey: "0040", clusterKey: "0040" },
    "clustered via: target → plan → parent",
  ],
  [
    "work-item-reviews",
    { entryKey: "0040", clusterKey: "0040" },
    "clustered via: target → work-item:0040",
  ],
  [
    "pr-reviews",
    { entryKey: "0040", clusterKey: "0040" },
    "clustered via: target → pr-description → parent",
  ],
  ["notes", { entryKey: null, clusterKey: null }, "clustered via: slug"],
  ["plans", { entryKey: null, clusterKey: null }, "clustered via: slug"],
];

describe("clusterViaLabel", () => {
  test.each(cases)("%s + clusterKey=%o → %s", (type, keys, expected) => {
    expect(
      clusterViaLabel(
        { type, clusterKey: keys.entryKey },
        { clusterKey: keys.clusterKey },
      ),
    ).toBe(expected);
  });
});
