import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { MarkdownRenderer } from "./MarkdownRenderer";
import type { Resolver } from "./wiki-link-plugin";

describe("MarkdownRenderer", () => {
  it("renders headings", () => {
    render(<MarkdownRenderer content="# Hello World" />);
    expect(
      screen.getByRole("heading", { level: 1, name: "Hello World" }),
    ).toBeInTheDocument();
  });

  it("renders GFM tables", async () => {
    // JSX attribute strings don't process \n escape sequences; use a JS
    // string expression so remark-gfm receives actual newline separators.
    render(<MarkdownRenderer content={"| A | B |\n|---|---|\n| x | y |"} />);
    expect(await screen.findByRole("table")).toBeInTheDocument();
  });

  it("renders a code block", () => {
    render(<MarkdownRenderer content="```js\nconsole.log('hi')\n```" />);
    expect(screen.getByText(/console\.log/)).toBeInTheDocument();
  });

  it("renders paragraphs", () => {
    render(<MarkdownRenderer content="Hello paragraph." />);
    expect(screen.getByText("Hello paragraph.")).toBeInTheDocument();
  });

  it("does not render raw HTML (XSS regression guard)", () => {
    // react-markdown defaults to escaping HTML. This test locks in that
    // default so enabling `rehype-raw` or `allowDangerousHtml` in future
    // requires deliberately breaking this test — at which point the
    // contributor is forced to add a sanitiser (e.g. rehype-sanitize).
    const { container } = render(
      <MarkdownRenderer content="<script>alert('xss')</script>" />,
    );
    expect(container.querySelector("script")).toBeNull();
    // The raw text survives as content; it's just not parsed as HTML.
    expect(container.textContent).toContain("<script>alert('xss')</script>");
  });

  it("does not render javascript: URLs in links (XSS regression guard)", () => {
    const { container } = render(
      <MarkdownRenderer content="[click]( javascript:alert(1) )" />,
    );
    const anchor = container.querySelector("a");
    // react-markdown's default urlTransform strips/rewrites dangerous schemes.
    // We assert no anchor with a javascript: href makes it into the DOM.
    expect(anchor?.getAttribute("href") ?? "").not.toMatch(/^\s*javascript:/i);
  });

  // ── Step 4.6 ───────────────────────────────────────────────────────────
  it("renders wiki-link as anchor when resolver returns kind=resolved", () => {
    const resolver: Resolver = () => ({
      kind: "resolved",
      href: "/library/decisions/ADR-0001-foo",
      title: "Example decision",
    });
    render(
      <MarkdownRenderer content="[[ADR-0001]]" resolveWikiLink={resolver} />,
    );
    const anchor = screen.getByRole("link", { name: "Example decision" });
    expect(anchor.getAttribute("href")).toBe("/library/decisions/ADR-0001-foo");
    expect(anchor.getAttribute("title")).toBe("[[ADR-0001]]");
  });

  // ── Step 4.7 ───────────────────────────────────────────────────────────
  it("renders unresolved-wiki-link span when resolver returns kind=unresolved", () => {
    const resolver: Resolver = () => ({ kind: "unresolved" });
    const { container } = render(
      <MarkdownRenderer content="[[ADR-9999]]" resolveWikiLink={resolver} />,
    );
    const span = container.querySelector("span.unresolved-wiki-link");
    expect(span).not.toBeNull();
    expect(span?.getAttribute("title")).toBe(
      "No matching ADR found for ID 9999",
    );
    expect(span?.textContent).toBe("[[ADR-9999]]");
    expect(container.querySelector("a")).toBeNull();
  });

  // ── Step 4.7b ──────────────────────────────────────────────────────────
  it("renders wiki-link-pending span when resolver returns kind=pending", () => {
    const resolver: Resolver = () => ({ kind: "pending" });
    const { container } = render(
      <MarkdownRenderer content="[[ADR-0001]]" resolveWikiLink={resolver} />,
    );
    const span = container.querySelector("span.wiki-link-pending");
    expect(span).not.toBeNull();
    expect(span?.getAttribute("title")).toBe("Loading reference…");
    expect(span?.textContent).toBe("[[ADR-0001]]");
    expect(container.querySelector("a")).toBeNull();
  });

  // ── Step 4.8 ───────────────────────────────────────────────────────────
  it("omits the plugin when resolveWikiLink is not provided", () => {
    const { container } = render(<MarkdownRenderer content="[[ADR-0001]]" />);
    expect(container.textContent).toContain("[[ADR-0001]]");
    expect(container.querySelector("a")).toBeNull();
    expect(container.querySelector("span.unresolved-wiki-link")).toBeNull();
    expect(container.querySelector("span.wiki-link-pending")).toBeNull();
  });

  // ── Step 4.10 ──────────────────────────────────────────────────────────
  it("sanitises resolver-supplied dangerous URL via urlTransform", () => {
    const resolver: Resolver = () => ({
      kind: "resolved",
      href: "javascript:alert(1)",
      title: "evil",
    });
    const { container } = render(
      <MarkdownRenderer content="[[ADR-0001]]" resolveWikiLink={resolver} />,
    );
    const anchor = container.querySelector("a");
    expect(anchor?.getAttribute("href") ?? "").not.toMatch(/^\s*javascript:/i);
  });

  describe("0095 — theme-reactive task-list checkboxes", () => {
    it("renders a tight GFM task list as token-driven boxes, not native inputs (0095)", () => {
      const { container } = render(
        <MarkdownRenderer content={"- [x] done\n- [ ] todo\n"} />,
      );
      // No native control survives.
      expect(container.querySelectorAll('input[type="checkbox"]')).toHaveLength(
        0,
      );
      // Two read-only checkbox boxes, state preserved for AT.
      const boxes = screen.getAllByRole("checkbox");
      expect(boxes).toHaveLength(2);
      expect(boxes[0]).toHaveAttribute("aria-checked", "true");
      expect(boxes[1]).toHaveAttribute("aria-checked", "false");
      boxes.forEach((b) => {
        expect(b).toHaveAttribute("aria-readonly", "true");
      });
      // Accessible name comes from the label (aria-labelledby).
      expect(screen.getByRole("checkbox", { name: "done" })).toBe(boxes[0]);
      expect(screen.getByRole("checkbox", { name: "todo" })).toBe(boxes[1]);
      // Tick present only on the checked box.
      expect(boxes[0].querySelector("svg")).not.toBeNull();
      expect(boxes[1].querySelector("svg")).toBeNull();
      // Label text preserved verbatim (children-survival, not just normalised name).
      const labels = container.querySelectorAll('li [class*="taskLabel"]');
      expect(labels[0].textContent?.trim()).toBe("done");
      expect(labels[1].textContent?.trim()).toBe("todo");
      // aria-labelledby ids are unique per item (useId, not a shared constant).
      const id0 = boxes[0].getAttribute("aria-labelledby");
      const id1 = boxes[1].getAttribute("aria-labelledby");
      expect(id0).toBeTruthy();
      expect(id0).not.toBe(id1);
      // The pure task list's <ul> gets the marker-removing tasklist class,
      // composed with (not clobbered by) the upstream contains-task-list class.
      expect(container.querySelector("ul")?.className ?? "").toMatch(
        /tasklist/,
      );
    });

    it("handles a LOOSE task list (input nested in <p>) with no native control (0095)", () => {
      // Blank line between items → loose list → mdast-util-to-hast keeps the <p>.
      const { container } = render(
        <MarkdownRenderer content={"- [x] done\n\n- [ ] todo\n"} />,
      );
      expect(container.querySelectorAll('input[type="checkbox"]')).toHaveLength(
        0,
      );
      const boxes = screen.getAllByRole("checkbox");
      expect(boxes).toHaveLength(2);
      expect(boxes[0]).toHaveAttribute("aria-checked", "true");
      expect(boxes[1]).toHaveAttribute("aria-checked", "false");
    });

    it("forwards a plain (non-task) list unchanged — no checkbox boxes (0095)", () => {
      const { container } = render(<MarkdownRenderer content={"- a\n- b\n"} />);
      expect(screen.queryAllByRole("checkbox")).toHaveLength(0);
      // Ordinary <li> still produced (markers come from the default <ul>).
      expect(container.querySelectorAll("ul > li")).toHaveLength(2);
    });

    it("renders task items as boxes even in a mixed list (0095)", () => {
      const { container } = render(
        <MarkdownRenderer content={"- [x] done\n- plain item\n"} />,
      );
      // Task item still boxed; native input still gone.
      expect(screen.getAllByRole("checkbox")).toHaveLength(1);
      expect(container.querySelectorAll('input[type="checkbox"]')).toHaveLength(
        0,
      );
      // A mixed list keeps its default markers — the <ul> must NOT get the
      // marker-removing tasklist class (the items.every(isTaskItem) branch).
      expect(container.querySelector("ul")?.className ?? "").not.toMatch(
        /tasklist/,
      );
    });
  });

  describe("Story 0076 AC4 — markdown pipeline behaviours", () => {
    it("renders a GFM table with thead/tbody/tr/td structure", () => {
      const { container } = render(
        <MarkdownRenderer
          content={"| H1 | H2 |\n|----|----|\n| a  | b  |\n"}
        />,
      );
      expect(container.querySelector("table thead tr th")?.textContent).toBe(
        "H1",
      );
      expect(container.querySelector("table tbody tr td")?.textContent).toBe(
        "a",
      );
    });

    it("routes a [[WORK-ITEM-NNNN]] wiki-link in body prose through the resolver", () => {
      const resolver: Resolver = (_prefix, id) => ({
        kind: "resolved",
        href: `/library/work-items/${id}`,
        title: `Work item ${id}`,
      });
      const { container } = render(
        <MarkdownRenderer
          content={"See [[WORK-ITEM-0042]] for context."}
          resolveWikiLink={resolver}
        />,
      );
      const anchor = container.querySelector("a");
      expect(anchor?.getAttribute("href")).toBe("/library/work-items/0042");
    });

    it("emits .hljs-keyword spans for an explicit language-python fenced code block", () => {
      const { container } = render(
        <MarkdownRenderer
          content={"```python\ndef foo():\n    return 1\n```"}
        />,
      );
      expect(
        container.querySelectorAll(".hljs-keyword").length,
      ).toBeGreaterThanOrEqual(1);
    });

    it("emits .hljs-keyword spans for an explicit language-typescript fenced code block", () => {
      const { container } = render(
        <MarkdownRenderer
          content={"```typescript\nconst x: number = 1\n```"}
        />,
      );
      expect(
        container.querySelectorAll(".hljs-keyword").length,
      ).toBeGreaterThanOrEqual(1);
    });

    it("renders a fenced code block and a [[WORK-ITEM-NNNN]] wiki-link in the same document without regression", () => {
      const resolver: Resolver = (_prefix, id) => ({
        kind: "resolved",
        href: `/library/work-items/${id}`,
        title: `Work item ${id}`,
      });
      const { container } = render(
        <MarkdownRenderer
          content={"See [[WORK-ITEM-0042]].\n\n```python\nx = 1\n```\n"}
          resolveWikiLink={resolver}
        />,
      );
      expect(
        container.querySelector('a[href="/library/work-items/0042"]'),
      ).not.toBeNull();
      expect(container.querySelector("pre code")).not.toBeNull();
    });

    it("does NOT resolve [[WORK-ITEM-NNNN]] inside an inline code span (verbatim pass-through)", () => {
      const resolver: Resolver = () => ({
        kind: "resolved",
        href: "/x",
        title: "x",
      });
      const { container } = render(
        <MarkdownRenderer
          content={"inline `[[WORK-ITEM-0042]]` should not resolve"}
          resolveWikiLink={resolver}
        />,
      );
      expect(container.querySelector('a[href="/x"]')).toBeNull();
      expect(container.textContent).toContain("[[WORK-ITEM-0042]]");
    });

    it("renders a header band with the language label for a labelled fence", () => {
      const { container } = render(
        <MarkdownRenderer content={"```python\nprint(1)\n```"} />,
      );
      const wrapper = container.querySelector('[data-language="python"]');
      expect(wrapper).not.toBeNull();
      // The label text is the verbatim fence label; CSS uppercases it.
      expect(wrapper!.textContent).toContain("python");
      // The <pre> still exists inside the wrapper.
      expect(wrapper!.querySelector("pre")).not.toBeNull();
    });

    it("renders a bare <pre> (no header) for an unlabelled fence", () => {
      const { container } = render(
        <MarkdownRenderer content={"```\nplain text\n```"} />,
      );
      expect(container.querySelector("[data-language]")).toBeNull();
      expect(container.querySelector("pre")).not.toBeNull();
    });

    it("renders an unknown-language fence with the base .hljs class (no thrown error)", () => {
      const { container } = render(
        <MarkdownRenderer content={"```klingon\nbatlh Daqawlu'taH\n```"} />,
      );
      const code = container.querySelector("pre code");
      expect(code).not.toBeNull();
      expect(code!.className).toMatch(/\bhljs\b/);
    });
  });

  // ── Step 4.9 ───────────────────────────────────────────────────────────
  describe("XSS regression guards still pass with plugin enabled", () => {
    const resolver: Resolver = () => ({
      kind: "resolved",
      href: "/safe",
      title: "safe",
    });

    it("does not render raw HTML", () => {
      const { container } = render(
        <MarkdownRenderer
          content="<script>alert('xss')</script>"
          resolveWikiLink={resolver}
        />,
      );
      expect(container.querySelector("script")).toBeNull();
    });

    it("does not render javascript: URLs in links", () => {
      const { container } = render(
        <MarkdownRenderer
          content="[click]( javascript:alert(1) )"
          resolveWikiLink={resolver}
        />,
      );
      const anchor = container.querySelector("a");
      expect(anchor?.getAttribute("href") ?? "").not.toMatch(
        /^\s*javascript:/i,
      );
    });
  });
});
