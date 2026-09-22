import presetIcons from '@unocss/preset-icons';
import transformerDirectives from '@unocss/transformer-directives';
import { defineConfig } from '@unocss/vite';

const baseSpaces = ['4xs', '3xs', '2xs', 'xs', 's', 'm', 'l', 'xl', '2xl', '3xl'];
const pairedSpaces = ['4xs-3xs', '3xs-2xs', '2xs-xs', 'xs-s', 's-m', 'm-l', 'l-xl', 'xl-2xl', '2xl-3xl'];
const extraSpaces = ['s-l'];
const spaceTokens = new Set([...baseSpaces, ...pairedSpaces, ...extraSpaces]);

const sideSuffixes: Record<string, string> = {
  '': '',
  t: '-top',
  b: '-bottom',
  s: '-inline-start',
  e: '-inline-end',
  x: '-inline',
  y: '-block',
};

export default defineConfig({
  presets: [presetIcons()],
  rules: [
    [
      /^([mp])([tbsexy]?)-([a-z0-9-]+)$/,
      ([, kind, side, size]) => {
        let suffix = sideSuffixes[side];
        if (suffix === undefined || !spaceTokens.has(size)) return;

        let property = kind === 'm' ? 'margin' : 'padding';
        return { [`${property}${suffix}`]: `var(--space-${size})` };
      },
    ],
  ],
  transformers: [transformerDirectives()],
});
