import { defer } from '@/e2e/deferred';
import { expect, test } from '@/e2e/helper';
import { http, HttpResponse } from 'msw';

test.describe('Acceptance | publish notifications', { tag: '@acceptance' }, () => {
  test('unsubscribe and resubscribe', async ({ page, msw }) => {
    let user = msw.db.user.create();
    await msw.authenticateAs(user);

    await page.goto('/settings/profile');
    await expect(page).toHaveURL('/settings/profile');
    await expect(page.locator('[data-test-notifications] input[type=checkbox]')).toBeChecked();

    await page.click('[data-test-notifications] input[type=checkbox]');
    await expect(page.locator('[data-test-notifications] input[type=checkbox]')).not.toBeChecked();

    await page.click('[data-test-notifications] button');
    user = msw.db.user.findFirst({ where: { id: { equals: user.id } } });
    expect(user.publishNotifications).toBe(false);

    await page.click('[data-test-notifications] input[type=checkbox]');
    await expect(page.locator('[data-test-notifications] input[type=checkbox]')).toBeChecked();

    await page.click('[data-test-notifications] button');
    user = msw.db.user.findFirst({ where: { id: { equals: user.id } } });
    expect(user.publishNotifications).toBe(true);
  });

  test('loading state', async ({ page, msw }) => {
    let user = msw.db.user.create();
    await msw.authenticateAs(user);

    let deferred = defer();
    msw.worker.use(http.put('/api/v1/users/:user_id', () => deferred.promise));

    await page.goto('/settings/profile');
    await expect(page).toHaveURL('/settings/profile');

    await page.click('[data-test-notifications] input[type=checkbox]');
    await page.click('[data-test-notifications] button');
    await expect(page.locator('[data-test-notifications] [data-test-spinner]')).toBeVisible();
    await expect(page.locator('[data-test-notifications] input[type=checkbox]')).toBeDisabled();
    await expect(page.locator('[data-test-notifications] button')).toBeDisabled();

    deferred.resolve();
    await expect(page.locator('[data-test-notifications] [data-test-spinner]')).not.toBeVisible();
    await expect(page.locator('[data-test-notification-message="error"]')).not.toBeVisible();
  });

  test('error state', async ({ page, msw }) => {
    let user = msw.db.user.create();
    await msw.authenticateAs(user);

    msw.worker.use(http.put('/api/v1/users/:user_id', () => HttpResponse.text('', { status: 500 })));

    await page.goto('/settings/profile');
    await expect(page).toHaveURL('/settings/profile');

    await page.click('[data-test-notifications] input[type=checkbox]');
    await page.click('[data-test-notifications] button');
    await expect(page.locator('[data-test-notifications] [data-test-spinner]')).not.toBeVisible();
    await expect(page.locator('[data-test-notification-message="error"]')).toHaveText(
      'Something went wrong while updating your notification settings. Please try again later!',
    );
  });
});
