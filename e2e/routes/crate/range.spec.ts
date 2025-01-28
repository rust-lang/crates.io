import { expect, test } from '@/e2e/helper';
import { http, HttpResponse } from 'msw';

test.describe('Route | crate.range', { tag: '@routes' }, () => {
  test('happy path', async ({ page, msw }) => {
    let crate = msw.db.crate.create({ name: 'foo' });
    msw.db.version.create({ crate, num: '1.0.0' });
    msw.db.version.create({ crate, num: '1.1.0' });
    msw.db.version.create({ crate, num: '1.2.0' });
    msw.db.version.create({ crate, num: '1.2.3' });

    await page.goto('/crates/foo/range/^1.1.0');
    await expect(page).toHaveURL(`/crates/foo/1.2.3`);
    await expect(page.locator('[data-test-crate-name]')).toHaveText('foo');
    await expect(page.locator('[data-test-crate-version]')).toHaveText('v1.2.3');
    await expect(page.locator('[data-test-notification-message]')).toHaveCount(0);
  });

  test('happy path with tilde range', async ({ page, msw }) => {
    let crate = msw.db.crate.create({ name: 'foo' });
    msw.db.version.create({ crate, num: '1.0.0' });
    msw.db.version.create({ crate, num: '1.1.0' });
    msw.db.version.create({ crate, num: '1.1.1' });
    msw.db.version.create({ crate, num: '1.2.0' });

    await page.goto('/crates/foo/range/~1.1.0');
    await expect(page).toHaveURL(`/crates/foo/1.1.1`);
    await expect(page.locator('[data-test-crate-name]')).toHaveText('foo');
    await expect(page.locator('[data-test-crate-version]')).toHaveText('v1.1.1');
    await expect(page.locator('[data-test-notification-message]')).toHaveCount(0);
  });

  test('happy path with cargo style and', async ({ page, msw }) => {
    let crate = msw.db.crate.create({ name: 'foo' });
    msw.db.version.create({ crate, num: '1.4.2' });
    msw.db.version.create({ crate, num: '1.3.4' });
    msw.db.version.create({ crate, num: '1.3.3' });
    msw.db.version.create({ crate, num: '1.2.6' });

    await page.goto('/crates/foo/range/>=1.3.0, <1.4.0');
    await expect(page).toHaveURL(`/crates/foo/1.3.4`);
    await expect(page.locator('[data-test-crate-name]')).toHaveText('foo');
    await expect(page.locator('[data-test-crate-version]')).toHaveText('v1.3.4');
    await expect(page.locator('[data-test-notification-message]')).toHaveCount(0);
  });

  test('ignores yanked versions if possible', async ({ page, msw }) => {
    let crate = msw.db.crate.create({ name: 'foo' });
    msw.db.version.create({ crate, num: '1.0.0' });
    msw.db.version.create({ crate, num: '1.1.0' });
    msw.db.version.create({ crate, num: '1.1.1' });
    msw.db.version.create({ crate, num: '1.2.0', yanked: true });

    await page.goto('/crates/foo/range/^1.0.0');
    await expect(page).toHaveURL(`/crates/foo/1.1.1`);
    await expect(page.locator('[data-test-crate-name]')).toHaveText('foo');
    await expect(page.locator('[data-test-crate-version]')).toHaveText('v1.1.1');
    await expect(page.locator('[data-test-notification-message]')).toHaveCount(0);
  });

  test('falls back to yanked version if necessary', async ({ page, msw }) => {
    let crate = msw.db.crate.create({ name: 'foo' });
    msw.db.version.create({ crate, num: '1.0.0', yanked: true });
    msw.db.version.create({ crate, num: '1.1.0', yanked: true });
    msw.db.version.create({ crate, num: '1.1.1', yanked: true });
    msw.db.version.create({ crate, num: '2.0.0' });

    await page.goto('/crates/foo/range/^1.0.0');
    await expect(page).toHaveURL(`/crates/foo/1.1.1`);
    await expect(page.locator('[data-test-crate-name]')).toHaveText('foo');
    await expect(page.locator('[data-test-crate-version]')).toHaveText('v1.1.1');
    await expect(page.locator('[data-test-notification-message]')).toHaveCount(0);
  });

  test('shows an error page if crate not found', async ({ page }) => {
    await page.goto('/crates/foo/range/^3');
    await expect(page).toHaveURL('/crates/foo/range/%5E3');
    await expect(page.locator('[data-test-404-page]')).toBeVisible();
    await expect(page.locator('[data-test-title]')).toHaveText('foo: Crate not found');
    await expect(page.locator('[data-test-go-back]')).toBeVisible();
    await expect(page.locator('[data-test-try-again]')).toHaveCount(0);
  });

  test('shows an error page if crate fails to load', async ({ page, msw }) => {
    msw.worker.use(http.get('/api/v1/crates/:crate_name', () => HttpResponse.json({}, { status: 500 })));

    await page.goto('/crates/foo/range/^3');
    await expect(page).toHaveURL('/crates/foo/range/%5E3');
    await expect(page.locator('[data-test-404-page]')).toBeVisible();
    await expect(page.locator('[data-test-title]')).toHaveText('foo: Failed to load crate data');
    await expect(page.locator('[data-test-go-back]')).toHaveCount(0);
    await expect(page.locator('[data-test-try-again]')).toBeVisible();
  });

  test('shows an error page if no match found', async ({ page, msw }) => {
    let crate = msw.db.crate.create({ name: 'foo' });
    msw.db.version.create({ crate, num: '1.0.0' });
    msw.db.version.create({ crate, num: '1.1.0' });
    msw.db.version.create({ crate, num: '1.1.1' });
    msw.db.version.create({ crate, num: '2.0.0' });

    await page.goto('/crates/foo/range/^3');
    await expect(page).toHaveURL('/crates/foo/range/%5E3');
    await expect(page.locator('[data-test-404-page]')).toBeVisible();
    await expect(page.locator('[data-test-title]')).toHaveText('foo: No matching version found for ^3');
    await expect(page.locator('[data-test-go-back]')).toBeVisible();
    await expect(page.locator('[data-test-try-again]')).toHaveCount(0);
  });

  test('shows an error page if versions fail to load', async ({ page, msw }) => {
    let crate = msw.db.crate.create({ name: 'foo' });
    msw.db.version.create({ crate, num: '3.2.1' });

    msw.worker.use(http.get('/api/v1/crates/:crate_name/versions', () => HttpResponse.json({}, { status: 500 })));

    await page.goto('/crates/foo/range/^3');
    await expect(page).toHaveURL('/crates/foo/range/%5E3');
    await expect(page.locator('[data-test-404-page]')).toBeVisible();
    await expect(page.locator('[data-test-title]')).toHaveText('foo: Failed to load version data');
    await expect(page.locator('[data-test-go-back]')).toHaveCount(0);
    await expect(page.locator('[data-test-try-again]')).toBeVisible();
  });
});
