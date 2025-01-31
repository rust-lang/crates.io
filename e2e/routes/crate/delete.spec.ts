import { defer } from '@/e2e/deferred';
import { expect, test } from '@/e2e/helper';
import { http, HttpResponse } from 'msw';

test.describe('Route: crate.delete', { tag: '@routes' }, () => {
  async function prepare(msw) {
    let user = msw.db.user.create();

    let crate = msw.db.crate.create({ name: 'foo' });
    msw.db.version.create({ crate });
    msw.db.crateOwnership.create({ crate, user });

    await msw.authenticateAs(user);
  }

  test('unauthenticated', async ({ msw, page }) => {
    let crate = msw.db.crate.create({ name: 'foo' });
    msw.db.version.create({ crate });

    await page.goto('/crates/foo/delete');
    await expect(page).toHaveURL('/crates/foo/delete');
    await expect(page.locator('[data-test-title]')).toHaveText('This page requires authentication');
    await expect(page.locator('[data-test-login]')).toBeVisible();
  });

  test('not an owner', async ({ msw, page }) => {
    let user1 = msw.db.user.create();
    await msw.authenticateAs(user1);

    let user2 = msw.db.user.create();
    let crate = msw.db.crate.create({ name: 'foo' });
    msw.db.version.create({ crate });
    msw.db.crateOwnership.create({ crate, user: user2 });

    await page.goto('/crates/foo/delete');
    await expect(page).toHaveURL('/crates/foo/delete');
    await expect(page.locator('[data-test-title]')).toHaveText('This page is only accessible by crate owners');
    await expect(page.locator('[data-test-go-back]')).toBeVisible();
  });

  test('happy path', async ({ msw, page, percy }) => {
    await prepare(msw);

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

    let crate = msw.db.crate.findFirst({ where: { name: { equals: 'foo' } } });
    expect(crate).toBeNull();
  });

  test('loading state', async ({ page, msw }) => {
    await prepare(msw);

    let deferred = defer();
    msw.worker.use(http.delete('/api/v1/crates/:name', () => deferred.promise));

    await page.goto('/crates/foo/delete');
    await page.fill('[data-test-reason]', "I don't need this crate anymore");
    await page.click('[data-test-confirmation-checkbox]');
    await page.click('[data-test-delete-button]');
    await expect(page.locator('[data-test-spinner]')).toBeVisible();
    await expect(page.locator('[data-test-confirmation-checkbox]')).toBeDisabled();
    await expect(page.locator('[data-test-delete-button]')).toBeDisabled();

    deferred.resolve();
    await expect(page).toHaveURL('/');
  });

  test('error state', async ({ page, msw }) => {
    await prepare(msw);

    let payload = { errors: [{ detail: 'only crates without reverse dependencies can be deleted after 72 hours' }] };
    msw.worker.use(http.delete('/api/v1/crates/:name', () => HttpResponse.json(payload, { status: 422 })));

    await page.goto('/crates/foo/delete');
    await page.fill('[data-test-reason]', "I don't need this crate anymore");
    await page.click('[data-test-confirmation-checkbox]');
    await page.click('[data-test-delete-button]');
    await expect(page).toHaveURL('/crates/foo/delete');

    let message = 'Failed to delete crate: only crates without reverse dependencies can be deleted after 72 hours';
    await expect(page.locator('[data-test-notification-message="error"]')).toHaveText(message);
  });
});
