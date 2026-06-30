import { setCustomExtension } from '@pierre/diffs';

/**
 * File names and extensions whose syntax highlighting language the `@pierre/diffs`
 * highlighter cannot infer on its own, mapped to their Shiki language id:
 */
const CUSTOM_EXTENSIONS: Record<string, string> = {
  'Cargo.lock': 'toml',
  'toml.orig': 'toml',
  svg: 'xml',
};

export function registerCustomExtensions() {
  for (let [name, lang] of Object.entries(CUSTOM_EXTENSIONS)) {
    setCustomExtension(name, lang);
  }
}
