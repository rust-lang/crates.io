import { fileURLToPath } from 'node:url';

import { includeIgnoreFile } from '@eslint/compat';
import js from '@eslint/js';
import prettier from 'eslint-config-prettier';
import preferLet from 'eslint-plugin-prefer-let';
import eslintPluginUnicorn from 'eslint-plugin-unicorn';
import { defineConfig } from 'eslint/config';
import globals from 'globals';
import ts from 'typescript-eslint';

const gitignorePath = fileURLToPath(new URL('.gitignore', import.meta.url));

export default defineConfig(
  eslintPluginUnicorn.configs.recommended,
  includeIgnoreFile(gitignorePath),
  // `crates/` (Rust workspace crates) and `svelte/` (SvelteKit app with its
  // own ESLint config) are not in `.gitignore`, but the root ESLint config
  // should not try to lint either of them.
  { ignores: ['crates/', 'svelte/'] },
  js.configs.recommended,
  ...ts.configs.recommended,
  prettier,
  {
    plugins: {
      'prefer-let': preferLet,
    },

    languageOptions: {
      globals: {
        ...globals.browser,
      },

      parser: ts.parser,
      sourceType: 'module',
    },

    rules: {
      // it's fine to use `return` without a value and rely on the implicit `undefined` return value
      'getter-return': 'off',

      'prefer-const': 'off',
      'prefer-let/prefer-let': 'error',

      // disabled because it seems unnecessary
      'unicorn/consistent-function-scoping': 'off',
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
      // disabled because `getElementById()` is faster than `querySelector("#id")`
      // and expresses intent more clearly
      'unicorn/prefer-query-selector': 'off',
      // disabled because of Sentry issues
      'unicorn/prefer-string-replace-all': 'off',
      // disabled because switch statements in JS are quite error-prone
      'unicorn/prefer-switch': 'off',
      // disabled because of false positives
      'unicorn/consistent-destructuring': 'off',
      // disabled because it does not play well with Svelte component naming conventions
      'unicorn/filename-case': 'off',
    },
  },

  // node files
  {
    files: ['eslint.config.mjs', 'script/**/*.mjs'],

    languageOptions: {
      globals: {
        ...Object.fromEntries(Object.entries(globals.browser).map(([key]) => [key, 'off'])),
        ...globals.node,
      },

      sourceType: 'script',
    },
  },
);
