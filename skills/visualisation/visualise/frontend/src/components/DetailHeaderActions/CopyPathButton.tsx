import { copyText } from "../../api/clipboard";
import { useToast } from "../../api/use-toast";
import { HeaderActionButton } from "./HeaderActionButton";

interface Props {
  /** Raw project-root-relative path (forward slashes, not percent-encoded). */
  relPath: string;
}

/** Copy / clipboard glyph (Feather "copy") — reads as "copy to clipboard",
 *  matching the renamed Copy path action. Decorative. */
const copyGlyph = (
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
    <title>Copy path</title>
    <rect x="9" y="9" width="13" height="13" rx="2" ry="2" />
    <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1" />
  </svg>
);

export function CopyPathButton({ relPath }: Props) {
  const { showToast } = useToast();

  async function onCopyPath() {
    // `copyText` resolves false when BOTH the Clipboard API and the
    // `execCommand` fallback fail; consume it so a failed copy surfaces the
    // persistent error toast rather than a misleading success confirmation.
    const ok = await copyText(relPath);
    if (ok) {
      showToast({
        heading: "Copied path to clipboard",
        message: `\`${relPath}\``,
        kind: "ok",
      });
    } else {
      showToast({
        heading: "Couldn’t copy path to clipboard",
        message: "",
        kind: "error",
      });
    }
  }

  return (
    <HeaderActionButton
      icon={copyGlyph}
      label="Copy path"
      title="Copy path"
      onClick={onCopyPath}
    />
  );
}
