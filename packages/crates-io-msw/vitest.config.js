import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: {
    setupFiles: 'vitest.setup.js',

    // The default Node.js environment does not support using relative paths
    // with `msw`, so we use `happy-dom` instead.
    environment: 'happy-dom',
  },
});
