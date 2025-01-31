import { expect, test } from '@/e2e/helper';

test.describe('Acceptance | crate deletion', { tag: '@acceptance' }, () => {
  test('happy path', async ({ page, msw }) => {
    let user = msw.db.user.create();
    await msw.authenticateAs(user);

    let crate = msw.db.crate.create({ name: 'foo' });
    msw.db.version.create({ crate });
    msw.db.crateOwnership.create({ crate, user });

    await page.goto('/crates/foo');
    await expect(page).toHaveURL('/crates/foo');
    await expect(page.locator('[data-test-settings-tab] a')).toBeVisible();

    await page.click('[data-test-settings-tab] a');
    await expect(page).toHaveURL('/crates/foo/settings');
    await expect(page.locator('[data-test-delete-button]')).toBeVisible();

    await page.click('[data-test-delete-button]');
    await expect(page).toHaveURL('/crates/foo/delete');
    await expect(page.locator('[data-test-title]')).toHaveText('Delete the foo crate?');
    await expect(page.locator('[data-test-delete-button]')).toBeDisabled();

    await page.fill('[data-test-reason]', "I don't need this crate anymore");
    await page.click('[data-test-confirmation-checkbox]');
    await expect(page.locator('[data-test-delete-button]')).toBeEnabled();

    await page.click('[data-test-delete-button]');
    await expect(page).toHaveURL('/');

    let message = 'Crate foo has been successfully deleted.';
    await expect(page.locator('[data-test-notification-message="success"]')).toHaveText(message);

    crate = msw.db.crate.findFirst({ where: { name: { equals: 'foo' } } });
    expect(crate).toBeNull();
  });
});
