import type { LogType, PluginOption, ProxyOptions } from 'vite';

import { fileURLToPath } from 'node:url';

import faviconIco from '@crates-io/favicon-ico';
import stripTestSelectors from '@crates-io/strip-test-selectors';
import adapter from '@sveltejs/adapter-static';
import { enhancedImages } from '@sveltejs/enhanced-img';
import { sveltekit } from '@sveltejs/kit/vite';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';
import UnoCSS from '@unocss/vite';
import { playwright } from '@vitest/browser-playwright';
import { msw } from 'msw/vite';
import { createLogger } from 'vite';
import { analyzer } from 'vite-bundle-analyzer';
import { defineConfig } from 'vitest/config';

const __TEST__ = Boolean(process.env.PLAYWRIGHT || process.env.VITEST);
const API_HOST = process.env.API_HOST ?? 'https://crates.io';

const proxyLogger = createLogger('info', { prefix: '[proxy]' });

const faviconSource = fileURLToPath(new URL('src/lib/assets/cargo.png', import.meta.url));

const preprocess = [vitePreprocess()];

if (process.env.NODE_ENV === 'production' && !process.env.PLAYWRIGHT && !process.env.VITEST) {
  preprocess.unshift(stripTestSelectors());
}

let sveltekitPlugin = sveltekit({
  // Consult https://svelte.dev/docs/kit/integrations
  // for more information about preprocessors
  preprocess,

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
      // @ts-expect-error SvelteKit's CSP types omit the valid wildcard source.
      'img-src': ['*', 'data:'],
      'object-src': ['none'],
    },
  },
});

const plugins: PluginOption[] = [UnoCSS(), enhancedImages(), faviconIco({ source: faviconSource }), sveltekitPlugin];
plugins.push({
  ...msw({ mode: 'worker-only' }),
  apply: 'serve',
});
if (process.env.BUNDLE_ANALYSIS) {
  plugins.push(analyzer({ analyzerMode: 'static' }));
}

let proxy: Record<string, string | ProxyOptions> | undefined;
if (!__TEST__) {
  proxy = {
    '/api': {
      target: API_HOST,
      changeOrigin: true,
      configure: proxy => {
        proxy.on('proxyRes', (proxyRes, req) => {
          let level: LogType = 'info';
          if ((proxyRes.statusCode ?? 0) >= 500) {
            level = 'error';
          } else if ((proxyRes.statusCode ?? 0) >= 400) {
            level = 'warn';
          }

          let msg = `${req.method} ${req.url} → ${proxyRes.statusCode} ${proxyRes.statusMessage}`;
          proxyLogger[level](msg, { timestamp: true });
        });
      },
    },
  };
}

export default defineConfig({
  define: {
    __TEST__,
  },

  plugins,

  server: {
    ...(process.env.DEV_DOCKER && { host: true }),
    proxy,
  },

  test: {
    expect: { requireAssertions: true },

    projects: [
      {
        extends: './vite.config.ts',

        test: {
          name: 'client',
          setupFiles: ['./src/test/setup-browser.ts'],

          browser: {
            enabled: true,
            provider: playwright(),
            instances: [{ browser: 'chromium', headless: true }],
          },

          include: ['src/**/*.svelte.{test,spec}.{js,ts}'],
          exclude: ['src/lib/server/**'],
        },
      },

      {
        extends: './vite.config.ts',

        test: {
          name: 'server',
          environment: 'node',
          include: ['src/**/*.{test,spec}.{js,ts}'],
          exclude: ['src/**/*.svelte.{test,spec}.{js,ts}'],
        },
      },
    ],
  },
});
