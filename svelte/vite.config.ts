import type { LogType } from 'vite';

import svg from '@poppanator/sveltekit-svg';
import { sveltekit } from '@sveltejs/kit/vite';
import { playwright } from '@vitest/browser-playwright';
import { createLogger } from 'vite';
import { defineConfig } from 'vitest/config';

const API_HOST = process.env.API_HOST ?? 'https://crates.io';

const proxyLogger = createLogger('info', { prefix: '[proxy]' });

export default defineConfig({
  plugins: [sveltekit(), svg()],

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

            const msg = `${req.method} ${req.url} → ${proxyRes.statusCode} ${proxyRes.statusMessage}`;
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
