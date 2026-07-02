import { byFileExtension, byFileName } from './icons.js';
import { spriteSheet } from './sprite.gen.js';

const SYMBOL_PREFIX = 'catppuccin';

function withPrefix(associations: Record<string, string>) {
  return Object.fromEntries(
    Object.entries(associations).map(([key, name]) => [key, `${SYMBOL_PREFIX}-${name}`] as const),
  );
}

export const FILE_TREE_ICONS = {
  set: 'none' as const,
  spriteSheet,
  remap: { 'file-tree-icon-file': 'catppuccin-file' },
  byFileExtension: withPrefix(byFileExtension),
  byFileName: withPrefix(byFileName),
};
