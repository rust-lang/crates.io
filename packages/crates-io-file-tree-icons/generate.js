// Sprite-sheet generator for the crates.io file tree icons.
//
// The `@iconify-json/catppuccin` set ships its glyphs in the "macchiato" flavor
// only. We want theme-aware icons, so every macchiato color is rewritten as a
// `light-dark(<latte>, <macchiato>)` value: latte for light mode, macchiato
// for dark mode. The mapping is built from `@catppuccin/palette` by matching the
// shared color name across both flavors.
//
// This module is only used at generation time (by `sprite.test.js`). The emitted
// `sprite.gen.js` is plain data with no runtime dependencies.

import { flavors } from '@catppuccin/palette';
import { icons as iconSet } from '@iconify-json/catppuccin';

const DEFAULT_SIZE = 16;
const SYMBOL_PREFIX = 'catppuccin';

const HEADER = `// This file is auto-generated. Do not edit manually.
//
// Run \`pnpm --filter @crates-io/file-tree-icons regenerate\` to update it.`;

/**
 * Generate the contents of `sprite.gen.js` for the given icon names.
 *
 * @param {Iterable<string>} iconNames icon names from `@iconify-json/catppuccin`
 * @returns {string} the module source, exporting a `spriteSheet` string
 */
export function generateSpriteModule(iconNames) {
  let replacements = buildColorReplacements();

  let symbols = [...new Set(iconNames)]
    .toSorted()
    .map(name => renderSymbol(name, replacements))
    .join('\n');

  let spriteSheet = `<svg xmlns="http://www.w3.org/2000/svg" aria-hidden="true" width="0" height="0">\n${symbols}\n</svg>`;

  return `${HEADER}\n\nexport const spriteSheet = ${toTemplateLiteral(spriteSheet)};\n`;
}

/**
 * Build the macchiato → `light-dark(latte, macchiato)` replacement table.
 *
 * @returns {Record<string, string>} lower-cased macchiato hex → replacement value
 */
function buildColorReplacements() {
  let table = {};
  for (let [name, macchiato] of Object.entries(flavors.macchiato.colors)) {
    let latte = flavors.latte.colors[name];
    table[macchiato.hex.toLowerCase()] = `light-dark(${latte.hex}, ${macchiato.hex})`;
  }
  return table;
}

function renderSymbol(name, replacements) {
  let icon = iconSet.icons[name];
  if (!icon) {
    throw new Error(`Unknown catppuccin icon: "${name}"`);
  }

  let left = icon.left ?? 0;
  let top = icon.top ?? 0;
  let width = icon.width ?? iconSet.width ?? DEFAULT_SIZE;
  let height = icon.height ?? iconSet.height ?? DEFAULT_SIZE;

  let body = recolor(icon.body, replacements);
  return `  <symbol id="${SYMBOL_PREFIX}-${name}" viewBox="${left} ${top} ${width} ${height}">${body}</symbol>`;
}

function recolor(body, replacements) {
  let hexes = Object.keys(replacements);
  let pattern = new RegExp(`(${hexes.join('|')})(?![0-9a-fA-F])`, 'gi');
  return body.replace(pattern, match => replacements[match.toLowerCase()]);
}

function toTemplateLiteral(value) {
  let escaped = value.replaceAll('\\', '\\\\').replaceAll('`', '\\`').replaceAll('${', '\\${');
  return `\`${escaped}\``;
}
