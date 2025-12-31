import { defineConfig, devices } from '@playwright/test';

/**
 * Read environment variables from file.
 * https://github.com/motdotla/dotenv
 */
// require('dotenv').config();

/**
 * TEST_APP controls which frontend app to test:
 * - 'ember' (default): Test the Ember.js app on port 4200
 * - 'svelte': Test the SvelteKit app on port 5173
 */
const TEST_APP = process.env.TEST_APP ?? 'ember';

const APP_CONFIG = {
  ember: {
    url: 'http://127.0.0.1:4200',
    command: 'pnpm start',
  },
  svelte: {
    url: 'http://localhost:4173',
    command: process.env.CI
      ? // on CI we compile once and then serve the static files, which is faster than running the dev server
        'npm run build && npm run preview'
      : // locally we run the dev server, which supports hot module replacement and is more convenient for development
        'npm run dev -- --port 4173',
    cwd: './svelte',
    env: { PLAYWRIGHT: '1' },
  },
} as const;

const appConfig = APP_CONFIG[TEST_APP] ?? APP_CONFIG.ember;

/**
 * See https://playwright.dev/docs/test-configuration.
 */
export default defineConfig({
  testDir: './e2e',
  /* Run tests in files in parallel */
  fullyParallel: true,
  /* Fail the build on CI if you accidentally left test.only in the source code. */
  forbidOnly: !!process.env.CI,
  /* Retry on CI only */
  retries: process.env.CI ? 2 : 0,
  /* Opt out of parallel tests on CI. */
  workers: process.env.CI ? 1 : undefined,
  /* Reporter to use. See https://playwright.dev/docs/test-reporters */
  reporter: process.env.CI
    ? [['github'], ['html', { outputFolder: 'playwright-report' }]]
    : [['html', { open: 'never' }]],
  /* Shared settings for all the projects below. See https://playwright.dev/docs/api/class-testoptions. */
  use: {
    /* Base URL to use in actions like `await page.goto('/')`. */
    baseURL: appConfig.url,

    /* Collect trace when retrying the failed test. See https://playwright.dev/docs/trace-viewer */
    trace: 'on-first-retry',

    /* Set a custom test id that is also compatible with `ember-test-selectors` */
    testIdAttribute: 'data-test-id',

    locale: 'en-US',
  },

  /* Configure projects for major browsers */
  projects: [
    {
      name: 'chromium',
      use: { ...devices['Desktop Chrome'] },
    },

    // {
    //   name: 'firefox',
    //   use: { ...devices['Desktop Firefox'] },
    // },
    //
    // {
    //   name: 'webkit',
    //   use: { ...devices['Desktop Safari'] },
    // },

    /* Test against mobile viewports. */
    // {
    //   name: 'Mobile Chrome',
    //   use: { ...devices['Pixel 5'] },
    // },
    // {
    //   name: 'Mobile Safari',
    //   use: { ...devices['iPhone 12'] },
    // },

    /* Test against branded browsers. */
    // {
    //   name: 'Microsoft Edge',
    //   use: { ...devices['Desktop Edge'], channel: 'msedge' },
    // },
    // {
    //   name: 'Google Chrome',
    //   use: { ...devices['Desktop Chrome'], channel: 'chrome' },
    // },
  ],

  /* Run your local dev server before starting the tests */
  webServer: {
    ...appConfig,
    reuseExistingServer: !process.env.CI,
    timeout: 5 * 60 * 1000,
  },
});
