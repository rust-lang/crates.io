import { expect, test } from '@/e2e/helper';

test.describe('Acceptance | publish notifications', { tag: '@acceptance' }, () => {
  test('unsubscribe and resubscribe', async ({ page, mirage }) => {
    await mirage.addHook(server => {
      let user = server.create('user');
      globalThis.user = user;
      authenticateAs(user);
    });

    await page.goto('/settings/profile');
    await expect(page).toHaveURL('/settings/profile');
    await expect(page.locator('[data-test-notifications] input[type=checkbox]')).toBeChecked();

    await page.click('[data-test-notifications] input[type=checkbox]');
    await expect(page.locator('[data-test-notifications] input[type=checkbox]')).not.toBeChecked();

    await page.click('[data-test-notifications] button');
    await page.waitForFunction(() => globalThis.user.reload().publishNotifications === false);

    await page.click('[data-test-notifications] input[type=checkbox]');
    await expect(page.locator('[data-test-notifications] input[type=checkbox]')).toBeChecked();

    await page.click('[data-test-notifications] button');
    await page.waitForFunction(() => globalThis.user.reload().publishNotifications === true);
  });

  test('loading state', async ({ page, mirage }) => {
    await mirage.addHook(server => {
      let user = server.create('user');
      authenticateAs(user);
      globalThis.user = user;

      globalThis.deferred = require('rsvp').defer();
      server.put('/api/v1/users/:user_id', globalThis.deferred.promise);
    });

    await page.goto('/settings/profile');
    await expect(page).toHaveURL('/settings/profile');

    await page.click('[data-test-notifications] input[type=checkbox]');
    await page.click('[data-test-notifications] button');
    await expect(page.locator('[data-test-notifications] [data-test-spinner]')).toBeVisible();
    await expect(page.locator('[data-test-notifications] input[type=checkbox]')).toBeDisabled();
    await expect(page.locator('[data-test-notifications] button')).toBeDisabled();

    await page.evaluate(async () => globalThis.deferred.resolve());
    await expect(page.locator('[data-test-notifications] [data-test-spinner]')).not.toBeVisible();
    await expect(page.locator('[data-test-notification-message="error"]')).not.toBeVisible();
  });

  test('error state', async ({ page, mirage }) => {
    await mirage.addHook(server => {
      server.logging = true;
      let user = server.create('user');
      authenticateAs(user);
      globalThis.user = user;

      server.put('/api/v1/users/:user_id', '', 500);
    });

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
