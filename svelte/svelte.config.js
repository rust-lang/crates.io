import adapter from '@sveltejs/adapter-static';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

import stripTestSelectors from './src/build/strip-test-selectors.js';

const preprocess = [vitePreprocess()];

if (process.env.NODE_ENV === 'production' && !process.env.PLAYWRIGHT && !process.env.VITEST) {
  preprocess.unshift(stripTestSelectors());
}

/** @type {import('@sveltejs/kit').Config} */
const config = {
  // Consult https://svelte.dev/docs/kit/integrations
  // for more information about preprocessors
  preprocess,

  kit: {
    adapter: adapter({
      // https://svelte.dev/docs/kit/single-page-apps#Usage recommends to
      // avoid using `index.html` as a fallback page, so we use `200.html` instead.
      fallback: '200.html',
      // Emit `.br` and `.gz` siblings for static assets at build time. The
      // backend's `static_or_continue` middleware serves them via
      // `ServeDir::precompressed_br().precompressed_gzip()`.
      precompress: true,
    }),
    paths: {
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
          // std-replacement dataset
          'https://rust-lang.github.io',
          // Trusted Publisher setup verifies the workflow file exists in the repo
          'https://raw.githubusercontent.com',
          // RustSec advisory lookup on the crate security tab
          'https://rustsec.org',
          // CDN that the `/api/v1/crates/{name}/{version}/readme` endpoint redirects to
          'https://static.crates.io',
          'https://static.staging.crates.io',
        ],
        'script-src': [
          'self',
          'unsafe-eval',
          // Hash of the inline `window.onerror` bootstrap script in `app.html`.
          // If the script content changes, regenerate this hash.
          'sha256-5Cz6+Mc7r7EqumpZ/iP8Bxa/U8yPvwbiANROmonMceg=',
        ],
        'style-src': ['self', 'unsafe-inline'],
        // `data:` is needed for UnoCSS `preset-icons`, which emits icons as
        // `mask-image: url(data:image/svg+xml,...)`. `*` does not cover the
        // `data:` scheme per the CSP spec.
        'img-src': ['*', 'data:'],
        'object-src': ['none'],
      },
    },
  },
};

export default config;
