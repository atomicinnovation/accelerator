const ADDRESS_MODULE_FILENAME = "/library-document-address.ts";

interface BoundaryRule {
  breach: string;
  isBrokenBy: (source: string) => boolean;
}

function containsAny(...needles: string[]): (source: string) => boolean {
  return (source) => needles.some((needle) => source.includes(needle));
}

const RULES_FOR_EVERY_OTHER_MODULE: BoundaryRule[] = [
  {
    breach: "assembles a /library/<type>/<document> target",
    isBrokenBy: (source) =>
      source.includes("/library/$type/$") ||
      /`\/library\/\$\{[^}]*\}\/\$\{/.test(source),
  },
  {
    breach: "defines a filename-stem helper",
    isBrokenBy: (source) =>
      /\b(?:function|const|let|var)\s+(?:filenameStemOf|fileSlugFromRelPath)\b/.test(
        source,
      ),
  },
  {
    breach: "compares a URL segment against an entry",
    isBrokenBy: (source) =>
      containsAny("=== documentId", "documentId ===")(source) ||
      /\.slug\s*===\s*params\.|\bparams\.\w+\s*===\s*\w+\.slug\b/.test(source),
  },
];

const DOC_TYPE_NAMES_THE_ADDRESS_MODULE_MUST_NOT_KNOW = [
  "TopicResearch",
  "DesignInventories",
  "topic-research",
  "design-inventories",
];

const RULES_FOR_THE_ADDRESS_MODULE: BoundaryRule[] =
  DOC_TYPE_NAMES_THE_ADDRESS_MODULE_MUST_NOT_KNOW.map((name) => ({
    breach: `names the ${name} doc type`,
    isBrokenBy: containsAny(name),
  }));

export function isLibraryDocumentAddressModule(path: string): boolean {
  return path.endsWith(ADDRESS_MODULE_FILENAME);
}

export function libraryAddressingViolations(
  sources: Record<string, string>,
): string[] {
  return Object.entries(sources).flatMap(([path, source]) => {
    const rules = isLibraryDocumentAddressModule(path)
      ? RULES_FOR_THE_ADDRESS_MODULE
      : RULES_FOR_EVERY_OTHER_MODULE;
    return rules
      .filter((rule) => rule.isBrokenBy(source))
      .map((rule) => `${path} ${rule.breach}`);
  });
}
