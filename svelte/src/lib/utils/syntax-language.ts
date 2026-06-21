/**
 * Resolves the Shiki language id used to highlight a source file, based on its
 * extension or a few well-known filenames.
 */

/** The default plain-text language used when no mapping matches. */
const PLAIN_TEXT_LANGUAGE = 'text';

/** Maps file extensions to Shiki language ids. */
const LANGUAGE_BY_EXTENSION: Record<string, string> = {
  bash: 'shellscript',
  c: 'c',
  cc: 'cpp',
  cjs: 'javascript',
  cpp: 'cpp',
  css: 'css',
  cxx: 'cpp',
  h: 'c',
  hpp: 'cpp',
  html: 'html',
  ini: 'ini',
  js: 'javascript',
  json: 'json',
  markdown: 'markdown',
  md: 'markdown',
  mjs: 'javascript',
  py: 'python',
  rs: 'rust',
  sh: 'shellscript',
  svg: 'xml',
  toml: 'toml',
  ts: 'typescript',
  xml: 'xml',
  yaml: 'yaml',
  yml: 'yaml',
};

/** Maps well-known filenames to Shiki language ids. */
const LANGUAGE_BY_FILENAME: Record<string, string> = {
  'Cargo.lock': 'toml',
  'Cargo.toml.orig': 'toml',
};

/** Resolves the Shiki language id for a file path, defaulting to plain text. */
export function languageForPath(path: string): string {
  let filename = path.slice(path.lastIndexOf('/') + 1);
  if (filename in LANGUAGE_BY_FILENAME) {
    return LANGUAGE_BY_FILENAME[filename];
  }

  let extension = filename.slice(filename.lastIndexOf('.') + 1).toLowerCase();
  return LANGUAGE_BY_EXTENSION[extension] ?? PLAIN_TEXT_LANGUAGE;
}
