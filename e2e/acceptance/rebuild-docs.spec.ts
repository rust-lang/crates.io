import { expect, test } from '@/e2e/helper';

test.describe('Acceptance | rebuild docs page', { tag: '@acceptance' }, () => {
  test('navigates to rebuild docs confirmation page', async ({ page, msw }) => {
    let user = await msw.db.user.create();
    await msw.authenticateAs(user);

    let crate = await msw.db.crate.create({ name: 'nanomsg' });
    await msw.db.crateOwnership.create({ crate, user });

    await msw.db.version.create({ crate, num: '0.1.0', created_at: '2017-01-01' });
    await msw.db.version.create({ crate, num: '0.2.0', created_at: '2018-01-01' });
    await msw.db.version.create({ crate, num: '0.3.0', created_at: '2019-01-01', rust_version: '1.69' });
    await msw.db.version.create({ crate, num: '0.2.1', created_at: '2020-01-01' });

    await page.goto('/crates/nanomsg/versions');
    await expect(page).toHaveURL('/crates/nanomsg/versions');

    await expect(page.locator('[data-test-version]')).toHaveCount(4);
    let versions = await page.locator('[data-test-version]').evaluateAll(el => el.map(it => it.dataset.testVersion));
    expect(versions).toEqual(['0.2.1', '0.3.0', '0.2.0', '0.1.0']);

    let v021 = page.locator('[data-test-version="0.2.1"]');
    await v021.locator('[data-test-actions-toggle]').click();
    await v021.getByRole('link', { name: 'Rebuild Docs' }).click();

    await expect(page).toHaveURL('/crates/nanomsg/0.2.1/rebuild-docs');
    await expect(page.locator('[data-test-title]')).toHaveText('Rebuild Documentation');
  });

  test('rebuild docs confirmation page shows crate info and allows confirmation', async ({ page, msw }) => {
    let user = await msw.db.user.create();
    await msw.authenticateAs(user);

    let crate = await msw.db.crate.create({ name: 'nanomsg' });
    await msw.db.crateOwnership.create({ crate, user });

    await msw.db.version.create({ crate, num: '0.2.1', created_at: '2020-01-01' });

    await page.goto('/crates/nanomsg/0.2.1/rebuild-docs');
    await expect(page).toHaveURL('/crates/nanomsg/0.2.1/rebuild-docs');

    await expect(page.locator('[data-test-title]')).toHaveText('Rebuild Documentation');
    await expect(page.locator('[data-test-crate-name]')).toHaveText('nanomsg');
    await expect(page.locator('[data-test-version-num]')).toHaveText('0.2.1');

    await page.getByRole('button', { name: 'Confirm Rebuild' }).click();

    let message = 'Docs rebuild task was enqueued successfully!';
    await expect(page.locator('[data-test-notification-message="success"]')).toHaveText(message);
    await expect(page).toHaveURL('/crates/nanomsg/versions');
  });

  test('rebuild docs confirmation page redirects non-owners to error page', async ({ page, msw }) => {
    let user = await msw.db.user.create();
    await msw.authenticateAs(user);

    let crate = await msw.db.crate.create({ name: 'nanomsg' });
    await msw.db.version.create({ crate, num: '0.2.1', created_at: '2020-01-01' });

    await page.goto('/crates/nanomsg/0.2.1/rebuild-docs');

    // Non-owners should be redirected to the catch-all error page
    await expect(page.getByText('This page is only accessible by crate owners')).toBeVisible();
  });

  test('rebuild docs confirmation page shows authentication error for unauthenticated users', async ({ page, msw }) => {
    let crate = await msw.db.crate.create({ name: 'nanomsg' });
    await msw.db.version.create({ crate, num: '0.2.1', created_at: '2020-01-01' });

    await page.goto('/crates/nanomsg/0.2.1/rebuild-docs');

    // Unauthenticated users should see authentication error
    await expect(page).toHaveURL('/crates/nanomsg/0.2.1/rebuild-docs');
    await expect(page.locator('[data-test-title]')).toHaveText('This page requires authentication');
    await expect(page.locator('[data-test-login]')).toBeVisible();
  });
});
