import { expect, test } from '@/e2e/helper';

test.describe('Route | crate.docs', { tag: '@routes' }, () => {
  test('redirects to the documentation URL', async ({ page, msw }) => {
    let crate = await msw.db.crate.create({ name: 'foo', documentation: 'https://foo.io/docs' });
    await msw.db.version.create({ crate, num: '1.0.0' });

    await page.route('https://foo.io/**', route => route.fulfill({ body: 'redirected' }));

    await page.goto('/crates/foo/docs');
    await expect(page).toHaveURL('https://foo.io/docs');
  });

  test('shows error and redirects to crate page if no documentation URL', async ({ page, msw }) => {
    let crate = await msw.db.crate.create({ name: 'foo' });
    await msw.db.version.create({ crate, num: '1.0.0' });

    await page.goto('/crates/foo/docs');
    await expect(page).toHaveURL('/crates/foo');
    await expect(page.locator('[data-test-notification-message="error"]')).toHaveText(
      'Crate does not supply a documentation URL',
    );
  });
});
