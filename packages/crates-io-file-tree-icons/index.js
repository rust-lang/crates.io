import { byFileExtension, byFileName } from './icons.js';
import { spriteSheet } from './sprite.gen.js';

const SYMBOL_PREFIX = 'catppuccin';

/** @param {Record<string, string>} associations */
function withPrefix(associations) {
  return Object.fromEntries(Object.entries(associations).map(([key, name]) => [key, `${SYMBOL_PREFIX}-${name}`]));
}

export const FILE_TREE_ICONS = {
  set: /** @type {const} */ ('none'),
  spriteSheet,
  remap: { 'file-tree-icon-file': 'catppuccin-file' },
  byFileExtension: withPrefix(byFileExtension),
  byFileName: withPrefix(byFileName),
};
