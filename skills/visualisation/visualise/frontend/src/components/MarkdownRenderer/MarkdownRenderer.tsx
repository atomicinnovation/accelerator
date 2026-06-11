import type { Element, ElementContent } from "hast";
import {
  Children,
  type ComponentPropsWithoutRef,
  isValidElement,
  type ReactNode,
  useId,
  useMemo,
} from "react";
import ReactMarkdown, { type Components } from "react-markdown";
import rehypeHighlight from "rehype-highlight";
import remarkGfm from "remark-gfm";
import { buildWikiLinkPattern } from "../../api/wiki-links";
import styles from "./MarkdownRenderer.module.css";
import { type Resolver, remarkWikiLinks } from "./wiki-link-plugin";

const DEFAULT_WIKI_LINK_PATTERN = buildWikiLinkPattern(null);

// Extracts the fence language label from a `<pre><code class="language-X …">`
// pair emitted by rehype-highlight. Returns `null` for unlabelled fences
// (which render as a bare `<pre>` with no header chrome).
function fenceLanguageOf(children: ReactNode): string | null {
  const first = Children.toArray(children).find(isValidElement) as
    | { props?: { className?: unknown } }
    | undefined;
  const className =
    typeof first?.props?.className === "string" ? first.props.className : "";
  const match = /\blanguage-(\S+)/.exec(className);
  return match?.[1] ?? null;
}

// White tick for the checked task-list box, modelled on the local CheckIcon
// in SortPill.tsx. `stroke="currentColor"` inherits the box's `color: #ffffff`
// so the tick paints white on the accent fill (mirrors FilterPill's checkmark).
function CheckIcon() {
  return (
    <svg
      width="11"
      height="11"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="3"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <path d="m5 12 5 5L20 7" />
    </svg>
  );
}

// The class mdast-util-to-hast stamps on task-list <li> nodes
// (handlers/list-item.js:52). The native <input> it injects (:43-48) is always
// `disabled` and sits either as a direct <li> child (tight lists) or inside a
// <p> (loose lists); `findCheckbox` searches both so the override is
// shape-agnostic.
const TASK_LIST_ITEM_CLASS = "task-list-item";

function isTaskItem(node: Element | undefined): boolean {
  const cls = node?.properties?.className;
  return Array.isArray(cls) && cls.includes(TASK_LIST_ITEM_CLASS);
}

// Recursively find the injected checkbox <input> (direct child for tight
// lists, nested in a <p> for loose lists) and return it.
function findCheckbox(children: ElementContent[]): Element | undefined {
  for (const child of children) {
    if (child.type !== "element") continue;
    if (child.tagName === "input") return child;
    const nested = findCheckbox(child.children); // child: Element ⇒ ElementContent[]
    if (nested) return nested;
  }
  return undefined;
}

// Always calls useId() at the top — no conditional hook (useId requires React
// 18+; the app pins React 19, so this is a no-op constraint). Props extend the
// <li> attribute set so forwarded props (and the hast `className`) are typed;
// we pull `className` out and compose it rather than letting `{...rest}`
// clobber the module-scoped classes (JSX last-wins).
//
// A11y (intentional, not incidental):
//  - `aria-readonly`, not `aria-disabled`: the box shows state but cannot be
//    changed. Several screen readers de-emphasise/skip disabled form controls,
//    which would suppress the very state announcement this override preserves.
//  - `role="checkbox"` lives on the box span, NOT the parent <li> (unlike
//    FilterPill, whose <li> is an interactive menu item). The markdown <li> is
//    a plain, non-interactive list item, so overriding its role would strip
//    list semantics and item count from AT.
function TaskListItem({
  checked,
  className,
  children,
  ...rest
}: { checked: boolean } & ComponentPropsWithoutRef<"li">) {
  const labelId = useId();
  // Compose: our module classes first, then the upstream `task-list-item`
  // class (if any) — so styles.task/styles.taskDone always apply.
  const liClass = [styles.task, checked && styles.taskDone, className]
    .filter(Boolean)
    .join(" ");
  return (
    <li className={liClass} {...rest}>
      {/* biome-ignore lint/a11y/useSemanticElements: deliberately a styled <span role="checkbox"> not a native <input> — the unit tests assert zero input[type=checkbox] and a custom .taskBox visual; semantic-input migration would change rendering */}
      {/* biome-ignore lint/a11y/useFocusableInteractive: a read-only (aria-readonly) markdown task checkbox is a display affordance, not an operable tab-stop, so it intentionally takes no tabIndex */}
      <span
        className={styles.taskBox}
        role="checkbox"
        aria-checked={checked}
        aria-readonly
        aria-labelledby={labelId}
      >
        {checked && <CheckIcon />}
      </span>
      {/* Block-level label so a loose list's <p> nests validly; it is the
          aria-labelledby target. */}
      <div id={labelId} className={styles.taskLabel}>
        {children}
      </div>
    </li>
  );
}

// `pre` renderer that adds the prototype's code-block header (the
// language label band) when the fence carries a `language-*` class.
// Unlabelled fences render as a bare `<pre>` so we don't add chrome
// where the prototype would not have it. The OS-window dots from the
// prototype's `.ac-codeblock__head` are intentionally omitted.
const MARKDOWN_COMPONENTS: Components = {
  pre({ children, node: _node, ...rest }) {
    const lang = fenceLanguageOf(children);
    if (!lang) {
      return <pre {...rest}>{children}</pre>;
    }
    return (
      <div className={styles.codeblock} data-language={lang}>
        <div className={styles.codeblockHead}>
          <span className={styles.codeblockLang}>{lang}</span>
        </div>
        <pre {...rest}>{children}</pre>
      </div>
    );
  },

  // Drop the GFM task-list checkbox wherever it sits (tight or loose),
  // independent of the <p>-unwrapping detail and of react-markdown's
  // element-type resolution. Scoped to the disabled checkbox mdast-util-to-hast
  // injects (list-item.js:46) so a future legitimate input is NOT swallowed.
  // INVARIANT: the `li` override below relies on this entry having removed the
  // native control from the children it wraps as the label.
  input({ node: _node, ...props }) {
    if (props.type === "checkbox" && props.disabled) return null;
    return <input {...props} />;
  },

  ul({ children, node, className, ...rest }) {
    // Mirror the prototype's `isTaskList = items.every(...)`: only a pure
    // task list drops its markers and gutter (a mixed list keeps markers).
    const items = (node?.children ?? []).filter(
      (c): c is Element => c.type === "element" && c.tagName === "li",
    );
    const isTaskList = items.length > 0 && items.every(isTaskItem);
    // Compose, never clobber: prepend styles.tasklist to the upstream
    // `contains-task-list` class (pulled out of rest) so a bare `{...rest}`
    // spread can't overwrite the module class (JSX last-wins).
    const ulClass = isTaskList
      ? [styles.tasklist, className].filter(Boolean).join(" ")
      : className;
    return (
      <ul className={ulClass} {...rest}>
        {children}
      </ul>
    );
  },

  li({ children, node, ...rest }) {
    if (!isTaskItem(node)) return <li {...rest}>{children}</li>;
    const checked = Boolean(
      findCheckbox((node?.children ?? []) as ElementContent[])?.properties
        ?.checked,
    );
    // `children` is the rendered label; the native <input> is already removed
    // by the `input` override above (INVARIANT), so no child-filtering is
    // needed and the loose-list <p> wrapper (if any) is preserved intact.
    return (
      <TaskListItem checked={checked} {...rest}>
        {children}
      </TaskListItem>
    );
  },
};

interface Props {
  content: string;
  /** Optional. When provided, body text matching the wiki-link
   *  bracket-shape (`[[ADR-NNNN]]` / `[[WORK-ITEM-NNNN]]`) is rewritten
   *  to anchors / pending-markers / unresolved-markers per the
   *  resolver's return kind. When omitted, no rewriting happens. */
  resolveWikiLink?: Resolver;
  /** The compiled wiki-link pattern to match against. When omitted and
   *  `resolveWikiLink` is provided, defaults to the default numeric-only
   *  pattern. Callers with a project-prefixed workspace should supply the
   *  pattern from `useWikiLinkResolver`. */
  wikiLinkPattern?: RegExp;
}

export function MarkdownRenderer({
  content,
  resolveWikiLink,
  wikiLinkPattern,
}: Props) {
  // Memoise the plugin tuple keyed on the resolver's and pattern's identity.
  // The resolver from `useWikiLinkResolver` is itself memoised (stable
  // across renders that don't change docs-cache state), so this tuple
  // is stable too — react-markdown short-circuits its pipeline re-run
  // for content-unchanged renders. When docs caches settle and the
  // resolver reference rotates, the tuple identity changes and the
  // pipeline re-runs, flipping pending markers to anchors.
  const effectivePattern = wikiLinkPattern ?? DEFAULT_WIKI_LINK_PATTERN;
  const remarkPlugins = useMemo(
    () =>
      resolveWikiLink
        ? ([
            remarkGfm,
            [remarkWikiLinks, effectivePattern, resolveWikiLink],
          ] as const)
        : ([remarkGfm] as const),
    [resolveWikiLink, effectivePattern],
  );
  return (
    <div className={styles.markdown}>
      <ReactMarkdown
        remarkPlugins={remarkPlugins as never}
        rehypePlugins={[rehypeHighlight]}
        components={MARKDOWN_COMPONENTS}
      >
        {content}
      </ReactMarkdown>
    </div>
  );
}
