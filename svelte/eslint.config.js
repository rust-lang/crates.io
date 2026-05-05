import { fileURLToPath } from 'node:url';

import { includeIgnoreFile } from '@eslint/compat';
import js from '@eslint/js';
import prettier from 'eslint-config-prettier';
import preferLet from 'eslint-plugin-prefer-let';
import storybook from 'eslint-plugin-storybook';
import svelte from 'eslint-plugin-svelte';
import { defineConfig } from 'eslint/config';
import globals from 'globals';
import ts from 'typescript-eslint';

import svelteConfig from './svelte.config.js';

const gitignorePath = fileURLToPath(new URL('./.gitignore', import.meta.url));

export default defineConfig(
  includeIgnoreFile(gitignorePath),
  js.configs.recommended,
  ...ts.configs.recommended,
  ...svelte.configs.recommended,
  prettier,
  ...svelte.configs.prettier,
  ...storybook.configs['flat/recommended'],
  {
    plugins: {
      'prefer-let': preferLet,
    },

    languageOptions: {
      globals: {
        ...globals.browser,
        ...globals.node,
        // Defined at build time by Vite (see `vite.config.ts`) and declared
        // ambiently in `src/app.d.ts`. ESLint's `no-undef` rule does not see
        // ambient `.d.ts` declarations, so we have to spell it out here.
        __TEST__: 'readonly',
      },
    },

    rules: {
      'prefer-const': 'off',
      'prefer-let/prefer-let': 'error',
    },
  },
  {
    files: ['**/*.svelte', '**/*.svelte.ts', '**/*.svelte.js'],

    languageOptions: {
      parserOptions: {
        projectService: true,
        extraFileExtensions: ['.svelte'],
        parser: ts.parser,
        svelteConfig,
      },
    },
  },
);
