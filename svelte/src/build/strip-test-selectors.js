import MagicString from 'magic-string';
import { parse } from 'svelte/compiler';
import { walk } from 'zimmerframe';

/**
 * Removes `data-test-*` and `data-testid` attributes from `.svelte` markup.
 *
 * Returns a Svelte preprocessor that always strips when invoked. The decision
 * to invoke it lives in `svelte.config.js`.
 *
 * @returns {import('svelte/compiler').PreprocessorGroup}
 */
export default function stripTestSelectors() {
  return {
    name: 'strip-test-selectors',

    markup({ content, filename }) {
      let ast = parse(content, { modern: true, filename });

      /** @type {Array<{ start: number, end: number }>} */
      let removals = [];

      walk(/** @type {import('svelte/compiler').AST.SvelteNode} */ (ast), null, {
        Attribute(node, ctx) {
          if (node.name.startsWith('data-test-') || node.name === 'data-testid') {
            removals.push({ start: node.start, end: node.end });
          }
          ctx.next();
        },
      });

      if (removals.length === 0) return undefined;

      let s = new MagicString(content);
      for (let { start, end } of removals) {
        s.remove(start, end);
      }

      return {
        code: s.toString(),
        map: s.generateMap({ hires: 'boundary', source: filename }),
      };
    },
  };
}
