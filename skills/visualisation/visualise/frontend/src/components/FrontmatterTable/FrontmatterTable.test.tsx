import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import type { Resolver } from "../MarkdownRenderer/wiki-link-plugin";
import { FrontmatterTable } from "./FrontmatterTable";
import css from "./FrontmatterTable.module.css?raw";

const passThroughResolver: Resolver = (prefix, id) => ({
  kind: "resolved",
  href: `/library/${prefix === "ADR" ? "decisions" : "work-items"}/${id}`,
  title: `${prefix}-${id}`,
});

const bareIdPattern = /\b(ADR|WORK-ITEM)-(\d+)\b/g;

const baseProps = {
  resolveWikiLink: passThroughResolver,
  bareIdPattern,
};

describe("FrontmatterTable", () => {
  it("renders one row per frontmatter key in source order", () => {
    const { container } = render(
      <FrontmatterTable
        {...baseProps}
        frontmatter={{
          work_item_id: "0078",
          title: "Detail-Page Frontmatter Table",
          date: "2026-05-21",
          author: "Toby Clemson",
          kind: "story",
          status: "ready",
          priority: "medium",
          parent: "",
          tags: ["design", "frontend"],
        }}
      />,
    );
    const rows = container.querySelectorAll("dt");
    expect(rows.length).toBe(9);
    const keys = Array.from(rows).map((dt) => dt.textContent);
    expect(keys).toEqual([
      "work_item_id",
      "title",
      "date",
      "author",
      "kind",
      "status",
      "priority",
      "parent",
      "tags",
    ]);
  });

  it("renders a dimmed em-dash for null, undefined, empty string, empty array, and empty object", () => {
    const { container } = render(
      <FrontmatterTable
        {...baseProps}
        frontmatter={
          {
            a: null,
            b: undefined,
            c: "",
            d: [],
            e: {},
          } as Record<string, unknown>
        }
      />,
    );
    const empties = container.querySelectorAll("[data-empty]");
    expect(empties.length).toBe(5);
    empties.forEach((dd) => {
      const dash = dd.querySelector("span");
      expect(dash?.textContent).toBe("—");
      expect(dash?.getAttribute("aria-hidden")).toBe("true");
    });
  });

  it("renders 0 and false as scalars, not as the empty dash", () => {
    render(
      <FrontmatterTable
        {...baseProps}
        frontmatter={{ archived: false, version: 0 }}
      />,
    );
    expect(screen.getByText("false")).toBeInTheDocument();
    expect(screen.getByText("0")).toBeInTheDocument();
  });

  it("renders comma-joined array values with comma separators between elements", () => {
    const { container } = render(
      <FrontmatterTable
        {...baseProps}
        frontmatter={{ tags: ["design", "frontend", "detail-page"] }}
      />,
    );
    const dd = container.querySelector("dd");
    expect(dd?.textContent).toBe("design, frontend, detail-page");
  });

  it("renders object values as JSON-serialised strings", () => {
    render(
      <FrontmatterTable
        {...baseProps}
        frontmatter={{ meta: { a: 1, b: "two" } }}
      />,
    );
    expect(screen.getByText('{"a":1,"b":"two"}')).toBeInTheDocument();
  });

  it("linkifies WORK-ITEM tokens embedded inside an object value", () => {
    const { container } = render(
      <FrontmatterTable
        {...baseProps}
        frontmatter={{ refs: { related: "WORK-ITEM-0041" } }}
      />,
    );
    const link = container.querySelector('a[href="/library/work-items/0041"]');
    expect(link).not.toBeNull();
    expect(link?.textContent).toBe("WORK-ITEM-0041");
  });

  it("linkifies a scalar value that is exactly a WORK-ITEM token", () => {
    render(
      <FrontmatterTable
        {...baseProps}
        frontmatter={{ parent: "WORK-ITEM-0041" }}
      />,
    );
    const link = screen.getByRole("link", { name: "WORK-ITEM-0041" });
    expect(link).toHaveAttribute("href", "/library/work-items/0041");
  });

  it("linkifies a WORK-ITEM token embedded in free text, leaving the rest plain", () => {
    const { container } = render(
      <FrontmatterTable
        {...baseProps}
        frontmatter={{ note: "see WORK-ITEM-0041 for context" }}
      />,
    );
    const link = container.querySelector('a[href="/library/work-items/0041"]');
    expect(link).not.toBeNull();
    expect(link?.textContent).toBe("WORK-ITEM-0041");
    expect(container.textContent).toContain("see ");
    expect(container.textContent).toContain(" for context");
  });

  it("linkifies array elements that match a token, leaving non-matching elements as text", () => {
    const { container } = render(
      <FrontmatterTable
        {...baseProps}
        frontmatter={{ refs: ["WORK-ITEM-0001", "misc", "ADR-0017"] }}
      />,
    );
    expect(
      container.querySelector('a[href="/library/work-items/0001"]'),
    ).not.toBeNull();
    expect(
      container.querySelector('a[href="/library/decisions/0017"]'),
    ).not.toBeNull();
    expect(container.textContent).toContain("misc");
  });

  it("does not linkify when the resolver returns unresolved", () => {
    const unresolved: Resolver = () => ({ kind: "unresolved" });
    const { container } = render(
      <FrontmatterTable
        {...baseProps}
        resolveWikiLink={unresolved}
        frontmatter={{ parent: "WORK-ITEM-9999" }}
      />,
    );
    expect(container.querySelector("a")).toBeNull();
    expect(screen.getByText("WORK-ITEM-9999")).toBeInTheDocument();
  });

  it("renders a pending span (matching the markdown body) when the resolver returns pending", () => {
    const pending: Resolver = () => ({ kind: "pending" });
    const { container } = render(
      <FrontmatterTable
        {...baseProps}
        resolveWikiLink={pending}
        frontmatter={{ parent: "WORK-ITEM-0041" }}
      />,
    );
    expect(container.querySelector("a")).toBeNull();
    const pendingSpan = container.querySelector('span[class*="pending"]');
    expect(pendingSpan).not.toBeNull();
    expect(pendingSpan?.textContent).toBe("WORK-ITEM-0041");
  });

  it("returns null when the frontmatter object is empty", () => {
    const { container } = render(
      <FrontmatterTable {...baseProps} frontmatter={{}} />,
    );
    expect(container.firstChild).toBeNull();
  });

  it('labels the container as "Document metadata" to match the malformed-banner wording', () => {
    render(<FrontmatterTable {...baseProps} frontmatter={{ kind: "story" }} />);
    expect(screen.getByLabelText("Document metadata")).toBeInTheDocument();
  });

  describe("CSS token contract", () => {
    it("uses --ac-bg-sunken for container background", () => {
      expect(css).toMatch(/background:\s*var\(--ac-bg-sunken\)/);
    });
    it("uses --ac-stroke for container border", () => {
      expect(css).toMatch(/border:\s*1px\s+solid\s+var\(--ac-stroke\)/);
    });
    it("uses --ac-font-mono (Fira Code) for table text", () => {
      expect(css).toMatch(/font-family:\s*var\(--ac-font-mono\)/);
    });
    it("uses --size-xxs-sm for table font-size (not a literal)", () => {
      expect(css).toMatch(/font-size:\s*var\(--size-xxs-sm\)/);
    });
    it("uses --ac-fg-faint for keys (design: fainter than body text)", () => {
      expect(css).toMatch(/\.key\s*\{[^}]*color:\s*var\(--ac-fg-faint\)/);
    });
    it("uses --ac-fg-muted for the empty-value dash and pending span", () => {
      expect(css).toMatch(/color:\s*var\(--ac-fg-muted\)/);
    });
    it("appends a colon after each key via ::after (decorative, not in DOM)", () => {
      expect(css).toMatch(/\.key::after\s*\{[^}]*content:\s*['"]:['"]/);
    });
    it("styles anchor children with an accent colour (not the inherited body colour)", () => {
      expect(css).toMatch(/\.value a\s*\{[^}]*color:\s*var\(--ac-accent\)/);
    });
  });
});
