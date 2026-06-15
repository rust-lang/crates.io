import { micromark } from 'micromark';

/**
 * Renders a Markdown string to an HTML string.
 *
 * Raw HTML in the input is escaped (micromark's `allowDangerousHtml` defaults
 * to `false`), so the result is safe to inject even when the Markdown comes
 * from an external source. No extensions are enabled, so only plain
 * CommonMark constructs are supported.
 */
export function renderSimpleMarkdown(markdown: string): string {
  return micromark(markdown);
}
