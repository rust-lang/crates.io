import { test, expect, Page, Route } from '@playwright/test';

test.describe('Acceptance | Read-only Mode', { tag: '@acceptance' }, () => {
  test.beforeEach(async ({ context }) => {
    // Block some assets requests for each test in this file.
    await context.route(/(css|png|woff|reload\.js)$/, route => route.abort());
  });

  test('notification is not shown for read-write mode', async ({ page }) => {
    await landingPageWithRoute(page, { status: 200, json: {} });

    await expect(page.locator('[data-test-notification-message="info"]')).toHaveCount(0);
  });

  test('notification is shown for read-only mode', async ({ page }) => {
    await landingPageWithRoute(page, { status: 200, json: { read_only: true } });

    await expect(page.locator('[data-test-notification-message="info"]')).toContainText('read-only mode');
  });

  test('server errors are handled gracefully', async ({ page }) => {
    await landingPageWithRoute(page, { status: 500, json: {} });

    await expect(page.locator('[data-test-notification-message="info"]')).toHaveCount(0);
    await checkSentryEventsNumber(page, 0);
  });

  test('client errors are reported on sentry', async ({ page }) => {
    await landingPageWithRoute(page, { status: 400, json: {} });

    await expect(page.locator('[data-test-notification-message="info"]')).toHaveCount(0);
    await checkSentryEventsNumber(page, 1);
    await checkSentryEventsHasName(page, 'AjaxError');
  });
});

async function landingPageWithRoute(page: Page, fulfill: Parameters<Route['fulfill']>[0]) {
  await page.route('**/*/api/v1/**/*', async route => {
    await route.fulfill(fulfill);
  });
  const apiResponse = page.waitForResponse('/api/v1/site_metadata');
  await page.goto('/');
  await apiResponse;
}

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
