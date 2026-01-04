import type { LogType } from 'vite';

import svg from '@poppanator/sveltekit-svg';
import { sveltekit } from '@sveltejs/kit/vite';
import { playwright } from '@vitest/browser-playwright';
import { createLogger } from 'vite';
import { analyzer } from 'vite-bundle-analyzer';
import { defineConfig } from 'vitest/config';

const API_HOST = process.env.API_HOST ?? 'https://crates.io';

const proxyLogger = createLogger('info', { prefix: '[proxy]' });

const plugins = [sveltekit(), svg()];
if (process.env.BUNDLE_ANALYSIS) {
  plugins.push(analyzer({ analyzerMode: 'static' }));
}

export default defineConfig({
  define: {
    __TEST__: Boolean(process.env.PLAYWRIGHT || process.env.VITEST),
  },

  plugins,

  server: {
    proxy: {
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
    },
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
