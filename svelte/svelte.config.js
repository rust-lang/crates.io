import adapter from '@sveltejs/adapter-static';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

/** @type {import('@sveltejs/kit').Config} */
const config = {
  // Consult https://svelte.dev/docs/kit/integrations
  // for more information about preprocessors
  preprocess: vitePreprocess(),

  kit: {
    adapter: adapter({
      // https://svelte.dev/docs/kit/single-page-apps#Usage recommends to
      // avoid using `index.html` as a fallback page, so we use `200.html` instead.
      fallback: '200.html',
    }),
    paths: {
      // We are serving the app from the `/svelte` subdirectory for now
      // to be able to serve it alongside the Ember.js app at `/`.
      // Use empty base path for tests (Vitest unit tests and Playwright e2e tests).
      base: process.env.VITEST || process.env.PLAYWRIGHT ? '' : '/svelte',
    },
  },
};

export default config;
