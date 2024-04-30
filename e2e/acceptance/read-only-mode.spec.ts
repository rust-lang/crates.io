import { test, expect } from '@/e2e/helper';
import { Page } from '@playwright/test';

test.describe('Acceptance | Read-only Mode', { tag: '@acceptance' }, () => {
  test.beforeEach(async ({ context }) => {
    // Block some assets requests for each test in this file.
    await context.route(/(css|png|woff|reload\.js)$/, route => route.abort());
  });

  test('notification is not shown for read-write mode', async ({ page }) => {
    await page.goto('/');

    await expect(page.locator('[data-test-notification-message="info"]')).toHaveCount(0);
  });

  test('notification is shown for read-only mode', async ({ page, mirage }) => {
    await mirage.addHook(server => {
      // @ts-expect-error
      server.get('/api/v1/site_metadata', { read_only: true });
    });
    await page.goto('/');

    await expect(page.locator('[data-test-notification-message="info"]')).toContainText('read-only mode');
  });

  test('server errors are handled gracefully', async ({ page, mirage }) => {
    await mirage.addHook(server => {
      // @ts-expect-error
      server.get('/api/v1/site_metadata', {}, 500);
    });
    await page.goto('/');

    await expect(page.locator('[data-test-notification-message="info"]')).toHaveCount(0);
    await checkSentryEventsNumber(page, 0);
  });

  test('client errors are reported on sentry', async ({ page, mirage }) => {
    await mirage.addHook(server => {
      // @ts-expect-error
      server.get('/api/v1/site_metadata', {}, 404);
    });
    await page.goto('/');

    await expect(page.locator('[data-test-notification-message="info"]')).toHaveCount(0);
    await checkSentryEventsNumber(page, 1);
    await checkSentryEventsHasName(page, 'AjaxError');
  });
});

async function checkSentryEventsNumber(page: Page, expected: number) {
  return await page.waitForFunction(e => {
    return window['__SENTRY_EVENTS']?.length ?? 0 === e;
  }, expected);
}

async function checkSentryEventsHasName(page: Page, expected: string) {
  return await page.waitForFunction(e => {
    return window['__SENTRY_EVENTS']?.map((e: Error) => e.name).includes(e);
  }, expected);
}
