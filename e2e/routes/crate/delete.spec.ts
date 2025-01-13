import { expect, test } from '@/e2e/helper';

test.describe('Route: crate.delete', { tag: '@routes' }, () => {
  async function prepare({ mirage }) {
    await mirage.addHook(server => {
      let user = server.create('user');

      let crate = server.create('crate', { name: 'foo' });
      server.create('version', { crate });
      server.create('crate-ownership', { crate, user });

      authenticateAs(user);
    });
  }

  test('unauthenticated', async ({ mirage, page }) => {
    await mirage.addHook(server => {
      let crate = server.create('crate', { name: 'foo' });
      server.create('version', { crate });
    });

    await page.goto('/crates/foo/delete');
    await expect(page).toHaveURL('/crates/foo/delete');
    await expect(page.locator('[data-test-title]')).toHaveText('This page requires authentication');
    await expect(page.locator('[data-test-login]')).toBeVisible();
  });

  test('not an owner', async ({ mirage, page }) => {
    await mirage.addHook(server => {
      let user1 = server.create('user');
      authenticateAs(user1);

      let user2 = server.create('user');
      let crate = server.create('crate', { name: 'foo' });
      server.create('version', { crate });
      server.create('crate-ownership', { crate, user: user2 });
    });

    await page.goto('/crates/foo/delete');
    await expect(page).toHaveURL('/crates/foo/delete');
    await expect(page.locator('[data-test-title]')).toHaveText('This page is only accessible by crate owners');
    await expect(page.locator('[data-test-go-back]')).toBeVisible();
  });

  test('happy path', async ({ mirage, page, percy }) => {
    await prepare({ mirage });

    await page.goto('/crates/foo/delete');
    await expect(page).toHaveURL('/crates/foo/delete');
    await expect(page.locator('[data-test-title]')).toHaveText('Delete the foo crate?');
    await percy.snapshot();

    await page.fill('[data-test-reason]', "I don't need this crate anymore");
    await expect(page.locator('[data-test-delete-button]')).toBeDisabled();
    await page.click('[data-test-confirmation-checkbox]');
    await expect(page.locator('[data-test-delete-button]')).toBeEnabled();
    await page.click('[data-test-delete-button]');

    await expect(page).toHaveURL('/');

    let message = 'Crate foo has been successfully deleted.';
    await expect(page.locator('[data-test-notification-message="success"]')).toHaveText(message);

    let crate = await page.evaluate(() => server.schema.crates.findBy({ name: 'foo' }));
    expect(crate).toBeNull();
  });

  test('loading state', async ({ page, mirage }) => {
    await prepare({ mirage });
    await mirage.addHook(server => {
      globalThis.deferred = require('rsvp').defer();
      server.delete('/api/v1/crates/foo', () => globalThis.deferred.promise);
    });

    await page.goto('/crates/foo/delete');
    await page.fill('[data-test-reason]', "I don't need this crate anymore");
    await page.click('[data-test-confirmation-checkbox]');
    await page.click('[data-test-delete-button]');
    await expect(page.locator('[data-test-spinner]')).toBeVisible();
    await expect(page.locator('[data-test-confirmation-checkbox]')).toBeDisabled();
    await expect(page.locator('[data-test-delete-button]')).toBeDisabled();

    await page.evaluate(async () => globalThis.deferred.resolve());
    await expect(page).toHaveURL('/');
  });

  test('error state', async ({ page, mirage }) => {
    await prepare({ mirage });
    await mirage.addHook(server => {
      let payload = { errors: [{ detail: 'only crates without reverse dependencies can be deleted after 72 hours' }] };
      server.delete('/api/v1/crates/foo', payload, 422);
    });

    await page.goto('/crates/foo/delete');
    await page.fill('[data-test-reason]', "I don't need this crate anymore");
    await page.click('[data-test-confirmation-checkbox]');
    await page.click('[data-test-delete-button]');
    await expect(page).toHaveURL('/crates/foo/delete');

    let message = 'Failed to delete crate: only crates without reverse dependencies can be deleted after 72 hours';
    await expect(page.locator('[data-test-notification-message="error"]')).toHaveText(message);
  });
});
