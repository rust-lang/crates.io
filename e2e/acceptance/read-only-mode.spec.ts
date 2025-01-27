import { test, expect, AppFixtures } from '@/e2e/helper';
import { http, HttpResponse } from 'msw';

test.describe('Acceptance | Read-only Mode', { tag: '@acceptance' }, () => {
  test.beforeEach(async ({ context }) => {
    // Block some assets requests for each test in this file.
    await context.route(/(css|png|woff|reload\.js)$/, route => route.abort());
  });

  test('notification is not shown for read-write mode', async ({ page }) => {
    await page.goto('/');

    await expect(page.locator('[data-test-notification-message="info"]')).toHaveCount(0);
  });

  test('notification is shown for read-only mode', async ({ page, msw }) => {
    let error = HttpResponse.json({}, { status: 500 });
    await msw.worker.use(http.put('/api/v1/me/crate_owner_invitations/:crate_id', () => error));

    await msw.worker.use(http.get('/api/v1/site_metadata', () => HttpResponse.json({ read_only: true })));
    await page.goto('/');

    await expect(page.locator('[data-test-notification-message="info"]')).toContainText('read-only mode');
  });

  test('server errors are handled gracefully', async ({ page, msw, ember }) => {
    await msw.worker.use(http.get('/api/v1/site_metadata', () => HttpResponse.json({}, { status: 500 })));
    await page.goto('/');

    await expect(page.locator('[data-test-notification-message="info"]')).toHaveCount(0);
    await checkSentryEventsNumber(ember, 0);
  });

  test('client errors are reported on sentry', async ({ page, msw, ember }) => {
    await msw.worker.use(http.get('/api/v1/site_metadata', () => HttpResponse.json({}, { status: 404 })));
    await page.goto('/');

    await expect(page.locator('[data-test-notification-message="info"]')).toHaveCount(0);
    await checkSentryEventsNumber(ember, 1);
    await checkSentryEventsHasName(ember, ['AjaxError']);
  });
});

async function checkSentryEventsNumber(ember: AppFixtures['ember'], expected: number) {
  let len = await ember.evaluate(owner => owner.lookup('service:sentry').events.length);
  expect(len).toBe(expected);
}

async function checkSentryEventsHasName(ember: AppFixtures['ember'], expected: string[]) {
  let events = await ember.evaluate(owner =>
    owner.lookup('service:sentry').events.map((e: { error: Error }) => e.error.name),
  );
  expect(events).toEqual(expected);
}
