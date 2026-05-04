import { expect, test } from '@/e2e/helper';
import { http, HttpResponse } from 'msw';

test.describe('Acceptance | Read-only Mode', { tag: '@acceptance' }, () => {
  test('notification is not shown for read-write mode', async ({ page }) => {
    await page.goto('/');

    await expect(page.locator('[data-test-notification-message="info"]')).toHaveCount(0);
  });

  test('notification is shown for read-only mode', async ({ page, msw }) => {
    let error = HttpResponse.json({}, { status: 500 });
    msw.worker.use(http.put('/api/v1/me/crate_owner_invitations/:crate_id', () => error));

    msw.worker.use(http.get('/api/v1/site_metadata', () => HttpResponse.json({ read_only: true })));
    await page.goto('/');

    await expect(page.locator('[data-test-notification-message="info"]')).toContainText('read-only mode');
  });

  test('server errors are handled gracefully', async ({ page, msw }) => {
    msw.worker.use(http.get('/api/v1/site_metadata', () => HttpResponse.json({}, { status: 500 })));
    await page.goto('/');

    await expect(page.locator('[data-test-notification-message="info"]')).toHaveCount(0);
  });

  test('client errors are reported on sentry', async ({ page, msw }) => {
    msw.worker.use(http.get('/api/v1/site_metadata', () => HttpResponse.json({}, { status: 404 })));
    await page.goto('/');

    await expect(page.locator('[data-test-notification-message="info"]')).toHaveCount(0);
  });

  test('banner message is shown when present', async ({ page, msw }) => {
    msw.worker.use(http.get('/api/v1/site_metadata', () => HttpResponse.json({ banner_message: 'test message' })));
    await page.goto('/');

    await expect(page.locator('[data-test-notification-message="info"]')).toContainText('test message');
  });

  test('banner message takes precedence over read-only mode', async ({ page, msw }) => {
    msw.worker.use(
      http.get('/api/v1/site_metadata', () => HttpResponse.json({ read_only: true, banner_message: 'test message' })),
    );
    await page.goto('/');

    await expect(page.locator('[data-test-notification-message="info"]')).toContainText('test message');
    await expect(page.locator('[data-test-notification-message="info"]')).not.toContainText('read-only mode');
  });
});
