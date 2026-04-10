import { AppFixtures, expect, test } from '@/e2e/helper';
import { http, HttpResponse } from 'msw';

const TEST_APP = process.env.TEST_APP ?? 'ember';

test.describe('Acceptance | Banner Messages', { tag: '@acceptance' }, () => {
  test('banner message can be dismissed and that is remembered', async ({ page, msw }) => {
    msw.worker.use(
      http.get('/api/v1/site_metadata', () => HttpResponse.json({ read_only: true, banner_message: 'test message' })),
    );
    await page.goto('/');

    await expect(page.locator('[data-test-notification-message="info"]')).toContainText('test message');
    await expect(page.locator('[data-test-notification-message="info"]')).not.toContainText('read-only mode');

    // Dismiss the notification.
    //
    // ember-cli-notification and the Svelte app create different element trees,
    // although they're functionally and visually identical to the user. Once we
    // remove Ember, this can be simplified to just the non-Ember
    // implementation.
    if (TEST_APP === 'ember') {
      // This feels fragile, but there isn't much else to hook onto here.
      await page.locator('[data-test-notification-message="info"] [title="Dismiss this notification"]').click();
    } else {
      await page.locator('[data-test-notification-message="info"] button').click();
    }

    // Verify that the notification disappeared after the animation.
    await expect(page.locator('[data-test-notification-message="info"]')).toHaveCount(0);

    // Reload the page and verify that the notification doesn't appear.
    await page.reload();
    await expect(page.locator('[data-test-notification-message="info"]')).toHaveCount(0);

    // Change the message and ensure the new message — and only the new message
    // — appears.
    msw.worker.use(
      http.get('/api/v1/site_metadata', () =>
        HttpResponse.json({ read_only: true, banner_message: 'second test message' }),
      ),
    );
    await page.reload();
    await expect(page.locator('[data-test-notification-message="info"]')).toHaveCount(1);
    await expect(page.locator('[data-test-notification-message="info"]')).toContainText('second test message');
  });
});
