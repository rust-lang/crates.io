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
      // Force absolute asset URLs under Playwright so that Percy's DOM
      // serializer captures hrefs that still resolve when the snapshot is
      // rendered at a different URL.
      ...(process.env.PLAYWRIGHT && { relative: false }),
    },
    prerender: {
      origin: `https://${process.env.DOMAIN_NAME ?? 'crates.io'}`,
    },
    csp: {
      mode: 'hash',
      directives: {
        'default-src': ['self'],
        'connect-src': [
          'self',
          // docs.rs build status check for crates
          'https://docs.rs',
          // Rust Playground top-100 crates list
          'https://play.rust-lang.org',
          // Trusted Publisher setup verifies the workflow file exists in the repo
          'https://raw.githubusercontent.com',
          // RustSec advisory lookup on the crate security tab
          'https://rustsec.org',
          // CDN that the `/api/v1/crates/{name}/{version}/readme` endpoint redirects to
          'https://static.crates.io',
          'https://static.staging.crates.io',
        ],
        'script-src': ['self', 'unsafe-eval'],
        // Fira Sans is loaded from the Mozilla CDN via `@import` in `global.css`
        'style-src': ['self', 'unsafe-inline', 'https://code.cdn.mozilla.net'],
        'font-src': ['https://code.cdn.mozilla.net'],
        'img-src': ['*'],
        'object-src': ['none'],
      },
    },
  },
};

export default config;
