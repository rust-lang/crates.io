import presetIcons from '@unocss/preset-icons';
import transformerDirectives from '@unocss/transformer-directives';
import { defineConfig } from '@unocss/vite';

export default defineConfig({
  presets: [presetIcons()],
  transformers: [transformerDirectives()],
});
