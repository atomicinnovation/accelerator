import { TopbarIconButton } from '../TopbarIconButton/TopbarIconButton'
import { copyText } from '../../api/clipboard'
import { useToast } from '../../api/use-toast'

interface Props {
  /** Raw project-root-relative path (forward slashes, not percent-encoded). */
  relPath: string
}

/** Clipboard glyph — two overlapping sheets, matching the topbar icon stroke
 *  style (16×16, `currentColor`, strokeWidth 2). Decorative. */
const copyGlyph = (
  <svg
    aria-hidden="true"
    width="16"
    height="16"
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    strokeWidth="2"
    strokeLinecap="round"
    strokeLinejoin="round"
  >
    <rect x="9" y="9" width="11" height="11" rx="2" />
    <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1" />
  </svg>
)

export function CopyPathButton({ relPath }: Props) {
  const { showToast } = useToast()

  async function onCopyPath() {
    // `copyText` resolves false when BOTH the Clipboard API and the
    // `execCommand` fallback fail; consume it so a failed copy surfaces the
    // persistent error toast rather than a misleading success confirmation.
    const ok = await copyText(relPath)
    if (ok) {
      showToast({
        heading: 'Copied path to clipboard',
        message: `\`${relPath}\``,
        kind: 'ok',
      })
    } else {
      showToast({
        heading: 'Couldn’t copy path to clipboard',
        message: '',
        kind: 'error',
      })
    }
  }

  return (
    <TopbarIconButton
      ariaLabel="Copy path"
      dataIcon="copy"
      title="Copy path"
      onClick={onCopyPath}
    >
      {copyGlyph}
    </TopbarIconButton>
  )
}
