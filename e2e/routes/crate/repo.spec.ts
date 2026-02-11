import { expect, test } from '@/e2e/helper';

test.describe('Route | crate.repo', { tag: '@routes' }, () => {
  test('redirects to the repository URL', async ({ page, msw }) => {
    let crate = await msw.db.crate.create({ name: 'foo', repository: 'https://github.com/foo/foo' });
    await msw.db.version.create({ crate, num: '1.0.0' });

    await page.route('https://github.com/**', route => route.fulfill({ body: 'redirected' }));

    await page.goto('/crates/foo/repo');
    await expect(page).toHaveURL('https://github.com/foo/foo');
  });

  test('shows error and redirects to crate page if no repository URL', async ({ page, msw }) => {
    let crate = await msw.db.crate.create({ name: 'foo' });
    await msw.db.version.create({ crate, num: '1.0.0' });

    await page.goto('/crates/foo/repo');
    await expect(page).toHaveURL('/crates/foo');
    await expect(page.locator('[data-test-notification-message="error"]')).toHaveText(
      'Crate does not supply a repository URL',
    );
  });
});
