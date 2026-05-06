import { fileURLToPath } from 'node:url';

import { includeIgnoreFile } from '@eslint/compat';
import js from '@eslint/js';
import prettier from 'eslint-config-prettier';
import preferLet from 'eslint-plugin-prefer-let';
import storybook from 'eslint-plugin-storybook';
import svelte from 'eslint-plugin-svelte';
import eslintPluginUnicorn from 'eslint-plugin-unicorn';
import { defineConfig } from 'eslint/config';
import globals from 'globals';
import ts from 'typescript-eslint';

import svelteConfig from './svelte.config.js';

const gitignorePath = fileURLToPath(new URL('../.gitignore', import.meta.url));
const repoRoot = fileURLToPath(new URL('..', import.meta.url));

export default defineConfig(
  eslintPluginUnicorn.configs.recommended,
  { ...includeIgnoreFile(gitignorePath), basePath: repoRoot },
  // Static assets contain vendored files like `mockServiceWorker.js` that
  // we should not lint.
  { ignores: ['static/'] },
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
      // it's fine to use `return` without a value and rely on the implicit `undefined` return value
      'getter-return': 'off',

      'prefer-const': 'off',
      'prefer-let/prefer-let': 'error',

      // disabled because it seems unnecessary
      'unicorn/consistent-function-scoping': 'off',
      // disabled because TypeScript already catches arity mismatches that
      // this rule is designed to flag (e.g. `arr.map(parseInt)`)
      'unicorn/no-array-callback-reference': 'off',
      'unicorn/explicit-length-check': ['error', { 'non-zero': 'not-equal' }],
      // disabled because `toReversed` is not "widely supported" yet
      'unicorn/no-array-reverse': 'off',
      // disabled because `toSorted` is not "widely supported" yet
      'unicorn/no-array-sort': 'off',
      // disabled because it is annoying in some cases...
      'unicorn/no-await-expression-member': 'off',
      // disabled because we need `null` since JSON has no `undefined`
      'unicorn/no-null': 'off',
      // disabled because this rule conflicts with prettier
      'unicorn/no-nested-ternary': 'off',
      // disabled because of unfixable false positives
      'unicorn/prevent-abbreviations': 'off',
      // disabled because we are targeting only browsers at the moment
      'unicorn/prefer-global-this': 'off',
      // disabled because we don't want to go all-in on ES6 modules for Node.js code yet
      'unicorn/prefer-module': 'off',
      // disabled because it seems unnecessary
      'unicorn/prefer-number-properties': 'off',
      // disabled because it seems unnecessary
      'unicorn/prefer-reflect-apply': 'off',
      // disabled because it seems unnecessary
      'unicorn/prefer-string-raw': 'off',
      // disabled because of Sentry issues
      'unicorn/prefer-string-replace-all': 'off',
      // disabled because `getElementById()` is faster than `querySelector("#id")`
      // and expresses intent more clearly
      'unicorn/prefer-query-selector': 'off',
      // disabled because of false positives on non-array `push()` methods
      'unicorn/prefer-single-call': 'off',
      // disabled because switch statements in JS are quite error-prone
      'unicorn/prefer-switch': 'off',
      // disabled because Svelte component `<script>` blocks instantiate
      // synchronously, so top-level await is not appropriate there
      'unicorn/prefer-top-level-await': 'off',
      // disabled because of false positives
      'unicorn/consistent-destructuring': 'off',
      // disabled because it does not play well with Svelte component naming conventions
      'unicorn/filename-case': 'off',
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
