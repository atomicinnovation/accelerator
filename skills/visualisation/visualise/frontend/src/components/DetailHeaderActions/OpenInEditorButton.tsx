import { useId } from "react";
import { buildEditorHref } from "../../api/editor-link";
import { useEditorConfig } from "../../api/use-editor-config";
import { HeaderActionButton } from "./HeaderActionButton";

interface Props {
  /** Canonical absolute filesystem path (`entry.path`) for `{abs}`. */
  absPath: string;
  /** Project-root-relative path (`entry.relPath`) for `{rel}`. */
  relPath: string;
}

/** Pencil glyph — the prototype DocPage's `edit` icon (Feather edit-2), size 13.
 *  Decorative. */
const editGlyph = (
  <svg
    width="13"
    height="13"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    strokeWidth="2"
    strokeLinecap="round"
    strokeLinejoin="round"
  >
    <title>Open in editor</title>
    <path d="M12 20h9" />
    <path d="M16.5 3.5a2.121 2.121 0 0 1 3 3L7 19l-4 1 1-4z" />
  </svg>
);

export function OpenInEditorButton({ absPath, relPath }: Props) {
  const { data } = useEditorConfig();
  // `useId` is called unconditionally (before any branch) so the hook order is
  // stable across the configured / disabled renders — the disabled branch wires
  // it to the visually-hidden description.
  const descId = useId();

  const href = data?.editor
    ? buildEditorHref({
        editor: data.editor,
        editorProject: data.editorProject,
        absPath,
        relPath,
      })
    : null;

  if (href) {
    return (
      <HeaderActionButton
        as="a"
        href={href}
        icon={editGlyph}
        label="Open in editor"
        title="Open in editor"
      />
    );
  }

  // Distinguish "not configured" from "configured but unrecognised" so the tooltip is
  // never misleading. A still-loading query (`data === undefined`) falls into the
  // unconfigured wording — a one-time, sub-second flip given `staleTime: Infinity`.
  const configuredEditor = data?.editor ?? null;
  // Truncate the echoed value so a long custom template can't push the guidance out of
  // a native tooltip; the full hint still reaches AT via the description element.
  const shown =
    configuredEditor && configuredEditor.length > 40
      ? `${configuredEditor.slice(0, 40)}…`
      : configuredEditor;
  const title = configuredEditor
    ? `visualiser.editor value “${shown}” was not recognised — set a preset key or a ` +
      `custom template containing {abs}/{rel}; see the visualiser.editor docs for the full list`
    : "Set visualiser.editor (or ACCELERATOR_VISUALISER_EDITOR) to enable opening files in your editor";

  return (
    <>
      <HeaderActionButton
        icon={editGlyph}
        label="Open in editor"
        disabled
        title={title}
        ariaDescribedBy={descId}
      />
      {/* Visually-hidden description: a native `title` is mouse-hover-only, so keyboard
          and screen-reader users would otherwise never get the enablement hint. */}
      <span id={descId} className="srOnly">
        {title}
      </span>
    </>
  );
}
