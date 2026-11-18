import { expect, it } from "vitest";
import { allocateWorkers } from "./worker-allocation";

it.each([
  [1, 1, 2],
  [3, 1, 2],
  [4, 2, 3],
  [16, 8, 9],
])("gives %i CPUs %i resolved-styles workers under a cap of %i", (cpus, resolvedStyles, total) => {
  expect(allocateWorkers(cpus)).toEqual({
    resolvedStyles,
    chromium: 1,
    total,
  });
});
