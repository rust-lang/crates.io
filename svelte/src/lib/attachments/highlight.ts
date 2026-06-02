import type { LanguageInput } from 'shiki/core';

import rust from '@shikijs/langs/rust';
import githubDark from '@shikijs/themes/github-dark';
import githubLight from '@shikijs/themes/github-light';
import { createHighlighterCoreSync } from 'shiki/core';
import { createJavaScriptRegexEngine } from 'shiki/engine/javascript';

/** Theme keys passed to shiki, rendered as inline `light-dark()` colors. */
const THEMES = { light: 'github-light', dark: 'github-dark' } as const;

interface Grammar {
  /** Language id passed to shiki, matching the loaded grammar's canonical name. */
  lang: string;
  /** Returns the grammar. Lazy entries import it into a separate bundle chunk. */
  load: () => LanguageInput;
}

/** A code block paired with the grammar it should be highlighted with. */
interface Block {
  el: Element;
  grammar: Grammar;
}

/**
 * Maps the `language-*` class names emitted by the backend Markdown renderer to
 * shiki grammars. Keys cover the allow-list in `crates_io_markdown` plus the
 * `clike`/`markup`/`rs` aliases. `mermaid` is intentionally absent because it
 * is rendered by a separate attachment.
 */
const GRAMMARS: Record<string, Grammar> = {
  bash: { lang: 'shellscript', load: () => import('@shikijs/langs/shellscript') },
  c: { lang: 'c', load: () => import('@shikijs/langs/c') },
  clike: { lang: 'c', load: () => import('@shikijs/langs/c') },
  cpp: { lang: 'cpp', load: () => import('@shikijs/langs/cpp') },
  csharp: { lang: 'csharp', load: () => import('@shikijs/langs/csharp') },
  glsl: { lang: 'glsl', load: () => import('@shikijs/langs/glsl') },
  go: { lang: 'go', load: () => import('@shikijs/langs/go') },
  ini: { lang: 'ini', load: () => import('@shikijs/langs/ini') },
  javascript: { lang: 'javascript', load: () => import('@shikijs/langs/javascript') },
  json: { lang: 'json', load: () => import('@shikijs/langs/json') },
  markup: { lang: 'xml', load: () => import('@shikijs/langs/xml') },
  protobuf: { lang: 'proto', load: () => import('@shikijs/langs/proto') },
  ruby: { lang: 'ruby', load: () => import('@shikijs/langs/ruby') },
  rs: { lang: 'rust', load: () => rust },
  rust: { lang: 'rust', load: () => rust },
  scss: { lang: 'scss', load: () => import('@shikijs/langs/scss') },
  sql: { lang: 'sql', load: () => import('@shikijs/langs/sql') },
  toml: { lang: 'toml', load: () => import('@shikijs/langs/toml') },
  xml: { lang: 'xml', load: () => import('@shikijs/langs/xml') },
  yaml: { lang: 'yaml', load: () => import('@shikijs/langs/yaml') },
};

/**
 * Long-lived highlighter, created synchronously so eagerly bundled grammars can
 * be applied before the first paint. Rust and the two themes are loaded upfront
 * since crate sources are predominantly Rust, while every other grammar is
 * loaded on demand.
 */
const highlighter = createHighlighterCoreSync({
  themes: [githubLight, githubDark],
  langs: [rust],
  engine: createJavaScriptRegexEngine(),
});

/** Returns the grammar for the first recognized `language-*` class, if any. */
function grammarFor(element: Element): Grammar | undefined {
  for (let cls of element.classList) {
    let match = /^language-(.+)$/.exec(cls);
    if (!match) continue;

    let grammar = GRAMMARS[match[1]];
    if (grammar) {
      return grammar;
    }
  }
  return undefined;
}

/** Replaces a code block's content with shiki's tokenized spans. */
function applyHighlight({ el, grammar }: Block) {
  let html = highlighter.codeToHtml(el.textContent ?? '', {
    lang: grammar.lang,
    themes: THEMES,
    defaultColor: 'light-dark()',
    colorsRendering: 'none',
  });

  // shiki wraps the output in its own `<pre><code>`, so we inject only the
  // inner tokens to preserve the existing `<pre><code>` styling.
  let template = document.createElement('template');
  template.innerHTML = html;
  let code = template.content.querySelector('code');
  if (code) {
    el.innerHTML = code.innerHTML;
  }
}

/**
 * Highlights the given code blocks. Blocks whose language is already loaded are
 * highlighted synchronously, so eagerly bundled grammars (Rust) render without a
 * flash of unstyled code. Any remaining languages are loaded and applied later.
 */
function highlight(elements: Element[], isCancelled: () => boolean) {
  let blocks: Block[] = [];
  for (let el of elements) {
    let grammar = grammarFor(el);
    if (grammar) {
      blocks.push({ el, grammar });
    }
  }

  let loaded = new Set(highlighter.getLoadedLanguages());
  let pending: Block[] = [];
  for (let block of blocks) {
    if (loaded.has(block.grammar.lang)) {
      applyHighlight(block);
    } else {
      pending.push(block);
    }
  }

  if (pending.length !== 0) {
    // The attachment must return its cleanup synchronously, so we don't await
    // the lazy grammar loads. `void` marks the promise as intentionally
    // unhandled. Stale runs are discarded via the `cancelled` flag.
    void loadPending(pending, isCancelled);
  }
}

/** Loads the grammars needed by `blocks`, then highlights them. */
async function loadPending(blocks: Block[], isCancelled: () => boolean) {
  let grammars = [...new Set(blocks.map(block => block.grammar))];
  await Promise.allSettled(grammars.map(grammar => highlighter.loadLanguage(grammar.load())));
  if (isCancelled()) return;

  for (let block of blocks) {
    applyHighlight(block);
  }
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
    highlight(elements, () => cancelled);

    return () => {
      cancelled = true;
    };
  };
}
