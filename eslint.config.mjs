import path from 'node:path';
import { fileURLToPath } from 'node:url';

import { FlatCompat } from '@eslint/eslintrc';
import js from '@eslint/js';
import emberEslintParser from 'ember-eslint-parser';
import preferLet from 'eslint-plugin-prefer-let';
import prettier from 'eslint-plugin-prettier';
import eslintPluginUnicorn from 'eslint-plugin-unicorn';
import globals from 'globals';

const filename = fileURLToPath(import.meta.url);
const dirname = path.dirname(filename);
const compat = new FlatCompat({
  baseDirectory: dirname,
  recommendedConfig: js.configs.recommended,
  allConfig: js.configs.all,
});

export default [
  eslintPluginUnicorn.configs.recommended,
  {
    ignores: [
      '.git/**/*',
      'crates/',
      'playwright-report/',
      'svelte/',
      'target/',
      'test-results/',
      'tmp/',
      // dependencies
      'node_modules/',
      // misc
      'coverage/',
      '!**/.*',
    ],
  },
  ...compat.extends('eslint:recommended', 'plugin:prettier/recommended'),
  {
    plugins: {
      'prefer-let': preferLet,
      prettier,
    },

    languageOptions: {
      globals: {
        ...globals.browser,
      },

      parser: emberEslintParser,
      ecmaVersion: 2018,
      sourceType: 'module',
    },

    rules: {
      // it's fine to use `return` without a value and rely on the implicit `undefined` return value
      'getter-return': 'off',

      'prefer-const': 'off',
      'prefer-let/prefer-let': 'error',

      'prettier/prettier': 'error',

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
      // disabled because of Sentry issues
      'unicorn/prefer-string-replace-all': 'off',
      // disabled because switch statements in JS are quite error-prone
      'unicorn/prefer-switch': 'off',
      // disabled because of false positives
      'unicorn/consistent-destructuring': 'off',
      'unicorn/filename-case': ['error', { case: 'kebabCase', ignore: ['^-'] }],
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

      ecmaVersion: 2018,
      sourceType: 'script',
    },
  },
];
