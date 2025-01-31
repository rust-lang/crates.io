import { expect, test } from '@/e2e/helper';

test.describe('Route | crate.version | model() hook', { tag: '@routes' }, () => {
  test.describe('with explicit version number in the URL', () => {
    test('shows yanked versions', async ({ page, msw }) => {
      let crate = msw.db.crate.create({ name: 'foo' });
      msw.db.version.create({ crate, num: '1.0.0' });
      msw.db.version.create({ crate, num: '1.2.3', yanked: true });
      msw.db.version.create({ crate, num: '2.0.0-beta.1' });

      await page.goto('/crates/foo/1.2.3');
      await expect(page).toHaveURL(`/crates/foo/1.2.3`);
      await expect(page.locator('[data-test-crate-name]')).toHaveText('foo');
      await expect(page.locator('[data-test-crate-version]')).toHaveText('v1.2.3');
      await expect(page.locator('[data-test-yanked]')).toBeVisible();
      await expect(page.locator('[data-test-docs]')).toBeVisible();
      await expect(page.locator('[data-test-install]')).toHaveCount(0);
      await expect(page.locator('[data-test-notification-message]')).toHaveCount(0);
    });

    test('shows error page for unknown versions', async ({ page, msw }) => {
      let crate = msw.db.crate.create({ name: 'foo' });
      msw.db.version.create({ crate, num: '1.0.0' });
      msw.db.version.create({ crate, num: '1.2.3', yanked: true });
      msw.db.version.create({ crate, num: '2.0.0-beta.1' });

      await page.goto('/crates/foo/2.0.0');
      await expect(page).toHaveURL(`/crates/foo/2.0.0`);
      await expect(page.locator('[data-test-404-page]')).toBeVisible();
      await expect(page.locator('[data-test-title]')).toHaveText('foo: Version 2.0.0 not found');
      await expect(page.locator('[data-test-go-back]')).toBeVisible();
      await expect(page.locator('[data-test-try-again]')).toHaveCount(0);
    });
  });
  test.describe('without version number in the URL', () => {
    test('defaults to the highest stable version', async ({ page, msw }) => {
      let crate = msw.db.crate.create({ name: 'foo' });
      msw.db.version.create({ crate, num: '1.0.0' });
      msw.db.version.create({ crate, num: '1.2.3', yanked: true });
      msw.db.version.create({ crate, num: '2.0.0-beta.1' });
      msw.db.version.create({ crate, num: '2.0.0' });

      await page.goto('/crates/foo');
      await expect(page).toHaveURL(`/crates/foo`);
      await expect(page.locator('[data-test-crate-name]')).toHaveText('foo');
      await expect(page.locator('[data-test-crate-version]')).toHaveText('v2.0.0');
      await expect(page.locator('[data-test-yanked]')).toHaveCount(0);
      await expect(page.locator('[data-test-docs]')).toBeVisible();
      await expect(page.locator('[data-test-install]')).toBeVisible();
      await expect(page.locator('[data-test-notification-message]')).toHaveCount(0);
    });

    test('defaults to the highest stable version, even if there are higher prereleases', async ({ page, msw }) => {
      let crate = msw.db.crate.create({ name: 'foo' });
      msw.db.version.create({ crate, num: '1.0.0' });
      msw.db.version.create({ crate, num: '1.2.3', yanked: true });
      msw.db.version.create({ crate, num: '2.0.0-beta.1' });

      await page.goto('/crates/foo');
      await expect(page).toHaveURL(`/crates/foo`);
      await expect(page.locator('[data-test-crate-name]')).toHaveText('foo');
      await expect(page.locator('[data-test-crate-version]')).toHaveText('v1.0.0');
      await expect(page.locator('[data-test-yanked]')).toHaveCount(0);
      await expect(page.locator('[data-test-docs]')).toBeVisible();
      await expect(page.locator('[data-test-install]')).toBeVisible();
      await expect(page.locator('[data-test-notification-message]')).toHaveCount(0);
    });

    test('defaults to the highest not-yanked version', async ({ page, msw }) => {
      let crate = msw.db.crate.create({ name: 'foo' });
      msw.db.version.create({ crate, num: '1.0.0', yanked: true });
      msw.db.version.create({ crate, num: '1.2.3', yanked: true });
      msw.db.version.create({ crate, num: '2.0.0-beta.1' });
      msw.db.version.create({ crate, num: '2.0.0-beta.2' });
      msw.db.version.create({ crate, num: '2.0.0', yanked: true });

      await page.goto('/crates/foo');
      await expect(page).toHaveURL(`/crates/foo`);
      await expect(page.locator('[data-test-crate-name]')).toHaveText('foo');
      await expect(page.locator('[data-test-crate-version]')).toHaveText('v2.0.0-beta.2');
      await expect(page.locator('[data-test-yanked]')).toHaveCount(0);
      await expect(page.locator('[data-test-docs]')).toBeVisible();
      await expect(page.locator('[data-test-install]')).toBeVisible();
      await expect(page.locator('[data-test-notification-message]')).toHaveCount(0);
    });

    test('if there are only yanked versions, it defaults to the latest version', async ({ page, msw }) => {
      let crate = msw.db.crate.create({ name: 'foo' });
      msw.db.version.create({ crate, num: '1.0.0', yanked: true });
      msw.db.version.create({ crate, num: '1.2.3', yanked: true });
      msw.db.version.create({ crate, num: '2.0.0-beta.1', yanked: true });

      await page.goto('/crates/foo');
      await expect(page).toHaveURL(`/crates/foo`);
      await expect(page.locator('[data-test-crate-name]')).toHaveText('foo');
      await expect(page.locator('[data-test-crate-version]')).toHaveText('v2.0.0-beta.1');
      await expect(page.locator('[data-test-yanked]')).toBeVisible();
      await expect(page.locator('[data-test-docs]')).toBeVisible();
      await expect(page.locator('[data-test-install]')).toHaveCount(0);
      await expect(page.locator('[data-test-notification-message]')).toHaveCount(0);
    });
  });
});
