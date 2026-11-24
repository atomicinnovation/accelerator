import { describe, expect, it } from "vitest";
import { libraryAddressingViolations } from "./library-addressing-boundary";

const ADDRESS_MODULE = "../api/library-document-address.ts";
const OTHER_MODULE = "../components/Thing/Thing.tsx";

function violationsOf(path: string, source: string): string[] {
  return libraryAddressingViolations({ [path]: source });
}

describe("libraryAddressingViolations", () => {
  it.each([
    ["a route-template target", 'const to = "/library/$type/$documentId";'],
    [
      "an interpolated document target",
      // biome-ignore lint/suspicious/noTemplateCurlyInString: the source under test is itself a template literal
      "const href = `/library/${type}/${documentId}`;",
    ],
    ["a stem helper", "function filenameStemOf(p: string) { return p; }"],
    [
      "the legacy stem helper",
      "export function fileSlugFromRelPath(p: string) { return p; }",
    ],
    ["a slug comparison", "entries.find((e) => e.slug === wanted);"],
    ["a URL-segment comparison", "entries.find((e) => e.id === documentId);"],
    [
      "a reversed URL-segment comparison",
      "entries.find((e) => documentId === e.id);",
    ],
  ])("reports %s outside the address module", (_, source) => {
    const violations = violationsOf(OTHER_MODULE, source);
    expect(violations).toHaveLength(1);
    expect(violations[0]).toContain(OTHER_MODULE);
  });

  it("allows the address module to assemble targets and resolve segments", () => {
    expect(
      violationsOf(
        ADDRESS_MODULE,
        [
          'const to = "/library/$type/$documentId";',
          "function filenameStemOf(p: string) { return p; }",
          "entries.find((e) => e.slug === documentId);",
        ].join("\n"),
      ),
    ).toEqual([]);
  });

  it.each([
    "TopicResearch",
    "DesignInventories",
    "topic-research",
    "design-inventories",
  ])("reports the address module naming %s", (name) => {
    const violations = violationsOf(ADDRESS_MODULE, `const t = "${name}";`);
    expect(violations).toHaveLength(1);
    expect(violations[0]).toContain(name);
  });

  it("allows a type path built from one interpolated segment", () => {
    expect(
      // biome-ignore lint/suspicious/noTemplateCurlyInString: the source under test is itself a template literal
      violationsOf(OTHER_MODULE, "const to = `/library/${dt.id}/`;"),
    ).toEqual([]);
  });

  it("allows other modules to name set and inventory doc types", () => {
    expect(violationsOf(OTHER_MODULE, 'const t = "topic-research";')).toEqual(
      [],
    );
  });
});
