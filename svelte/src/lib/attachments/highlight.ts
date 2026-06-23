import { createBundledHighlighter, createSingletonShorthands } from 'shiki/core';
import { createJavaScriptRegexEngine } from 'shiki/engine/javascript';
import { bundledLanguages } from 'shiki/langs';

/** Theme keys passed to shiki, rendered as inline `light-dark()` colors. */
const THEMES = { light: 'github-light', dark: 'github-dark' } as const;

/**
 * Prism.js `language-*` names emitted by the backend Markdown renderer that
 * shiki does not recognize, mapped to their shiki language id.
 */
const ALIASES: Record<string, string> = {
  clike: 'c',
  markup: 'xml',
};

const { codeToHtml } = createSingletonShorthands(
  createBundledHighlighter({
    langs: bundledLanguages,
    themes: {
      'github-light': () => import('@shikijs/themes/github-light'),
      'github-dark': () => import('@shikijs/themes/github-dark'),
    },
    engine: () => createJavaScriptRegexEngine(),
  }),
);

/** Returns the shiki language id for the first highlightable `language-*` class. */
function langFor(element: Element): string | undefined {
  for (let cls of element.classList) {
    let match = /^language-(.+)$/.exec(cls);
    if (!match) continue;

    let lang = ALIASES[match[1]] ?? match[1];
    if (Object.hasOwn(bundledLanguages, lang)) {
      return lang;
    }
  }
  return undefined;
}

/** Replaces a code block's content with shiki's tokenized spans. */
async function applyHighlight(el: Element, lang: string, isCancelled: () => boolean) {
  let html = await codeToHtml(el.textContent ?? '', {
    lang,
    themes: THEMES,
    defaultColor: 'light-dark()',
    colorsRendering: 'none',
  });
  if (isCancelled()) return;

  // shiki wraps the output in its own `<pre><code>`, so we inject only the
  // inner tokens to preserve the existing `<pre><code>` styling.
  let template = document.createElement('template');
  template.innerHTML = html;
  let code = template.content.querySelector('code');
  if (code) {
    el.innerHTML = code.innerHTML;
  }
}

/** Highlights the given code blocks, loading each grammar on demand. */
async function highlight(elements: Element[], isCancelled: () => boolean) {
  let pending = [];
  for (let el of elements) {
    let lang = langFor(el);
    if (lang) {
      pending.push(applyHighlight(el, lang, isCancelled));
    }
  }
  await Promise.allSettled(pending);
}

/**
 * Attachment that applies syntax highlighting to code blocks using shiki.
 *
 * Without a `selector` the attached element is highlighted directly. Otherwise
 * every match of `selector` within it is highlighted.
 */
export function highlightSyntax(html?: string, selector?: string) {
  return (element: Element) => {
    // Ensure that the attachment is re-run when `html` changes
    void html;

    let cancelled = false;
    let elements = selector ? [...element.querySelectorAll(selector)] : [element];
    void highlight(elements, () => cancelled);

    return () => {
      cancelled = true;
    };
  };
}
