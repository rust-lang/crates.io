import { test, expect } from '@/e2e/helper';

test.describe('Route | crate.range', { tag: '@routes' }, () => {
  test('happy path', async ({ page, mirage }) => {
    await mirage.addHook(server => {
      let crate = server.create('crate', { name: 'foo' });
      server.create('version', { crate, num: '1.0.0' });
      server.create('version', { crate, num: '1.1.0' });
      server.create('version', { crate, num: '1.2.0' });
      server.create('version', { crate, num: '1.2.3' });
    });

    await page.goto('/crates/foo/range/^1.1.0');
    await expect(page).toHaveURL(`/crates/foo/1.2.3`);
    await expect(page.locator('[data-test-crate-name]')).toHaveText('foo');
    await expect(page.locator('[data-test-crate-version]')).toHaveText('v1.2.3');
    await expect(page.locator('[data-test-notification-message]')).toHaveCount(0);
  });

  test('happy path with tilde range', async ({ page, mirage }) => {
    await mirage.addHook(server => {
      let crate = server.create('crate', { name: 'foo' });
      server.create('version', { crate, num: '1.0.0' });
      server.create('version', { crate, num: '1.1.0' });
      server.create('version', { crate, num: '1.1.1' });
      server.create('version', { crate, num: '1.2.0' });
    });

    await page.goto('/crates/foo/range/~1.1.0');
    await expect(page).toHaveURL(`/crates/foo/1.1.1`);
    await expect(page.locator('[data-test-crate-name]')).toHaveText('foo');
    await expect(page.locator('[data-test-crate-version]')).toHaveText('v1.1.1');
    await expect(page.locator('[data-test-notification-message]')).toHaveCount(0);
  });

  test('happy path with cargo style and', async ({ page, mirage }) => {
    await mirage.addHook(server => {
      let crate = server.create('crate', { name: 'foo' });
      server.create('version', { crate, num: '1.4.2' });
      server.create('version', { crate, num: '1.3.4' });
      server.create('version', { crate, num: '1.3.3' });
      server.create('version', { crate, num: '1.2.6' });
    });

    await page.goto('/crates/foo/range/>=1.3.0, <1.4.0');
    await expect(page).toHaveURL(`/crates/foo/1.3.4`);
    await expect(page.locator('[data-test-crate-name]')).toHaveText('foo');
    await expect(page.locator('[data-test-crate-version]')).toHaveText('v1.3.4');
    await expect(page.locator('[data-test-notification-message]')).toHaveCount(0);
  });

  test('ignores yanked versions if possible', async ({ page, mirage }) => {
    await mirage.addHook(server => {
      let crate = server.create('crate', { name: 'foo' });
      server.create('version', { crate, num: '1.0.0' });
      server.create('version', { crate, num: '1.1.0' });
      server.create('version', { crate, num: '1.1.1' });
      server.create('version', { crate, num: '1.2.0', yanked: true });
    });

    await page.goto('/crates/foo/range/^1.0.0');
    await expect(page).toHaveURL(`/crates/foo/1.1.1`);
    await expect(page.locator('[data-test-crate-name]')).toHaveText('foo');
    await expect(page.locator('[data-test-crate-version]')).toHaveText('v1.1.1');
    await expect(page.locator('[data-test-notification-message]')).toHaveCount(0);
  });

  test('falls back to yanked version if necessary', async ({ page, mirage }) => {
    await mirage.addHook(server => {
      let crate = server.create('crate', { name: 'foo' });
      server.create('version', { crate, num: '1.0.0', yanked: true });
      server.create('version', { crate, num: '1.1.0', yanked: true });
      server.create('version', { crate, num: '1.1.1', yanked: true });
      server.create('version', { crate, num: '2.0.0' });
    });

    await page.goto('/crates/foo/range/^1.0.0');
    await expect(page).toHaveURL(`/crates/foo/1.1.1`);
    await expect(page.locator('[data-test-crate-name]')).toHaveText('foo');
    await expect(page.locator('[data-test-crate-version]')).toHaveText('v1.1.1');
    await expect(page.locator('[data-test-notification-message]')).toHaveCount(0);
  });

  test('shows an error page if crate not found', async ({ page, mirage }) => {
    await page.goto('/crates/foo/range/^3');
    await expect(page).toHaveURL('/crates/foo/range/%5E3');
    await expect(page.locator('[data-test-404-page]')).toBeVisible();
    await expect(page.locator('[data-test-title]')).toHaveText('foo: Crate not found');
    await expect(page.locator('[data-test-go-back]')).toBeVisible();
    await expect(page.locator('[data-test-try-again]')).toHaveCount(0);
  });

  test('shows an error page if crate fails to load', async ({ page, mirage }) => {
    await mirage.addHook(server => {
      server.get('/api/v1/crates/:crate_name', {}, 500);
    });

    await page.goto('/crates/foo/range/^3');
    await expect(page).toHaveURL('/crates/foo/range/%5E3');
    await expect(page.locator('[data-test-404-page]')).toBeVisible();
    await expect(page.locator('[data-test-title]')).toHaveText('foo: Failed to load crate data');
    await expect(page.locator('[data-test-go-back]')).toHaveCount(0);
    await expect(page.locator('[data-test-try-again]')).toBeVisible();
  });

  test('shows an error page if no match found', async ({ page, mirage }) => {
    await mirage.addHook(server => {
      let crate = server.create('crate', { name: 'foo' });
      server.create('version', { crate, num: '1.0.0' });
      server.create('version', { crate, num: '1.1.0' });
      server.create('version', { crate, num: '1.1.1' });
      server.create('version', { crate, num: '2.0.0' });
    });
    await page.goto('/crates/foo/range/^3');
    await expect(page).toHaveURL('/crates/foo/range/%5E3');
    await expect(page.locator('[data-test-404-page]')).toBeVisible();
    await expect(page.locator('[data-test-title]')).toHaveText('foo: No matching version found for ^3');
    await expect(page.locator('[data-test-go-back]')).toBeVisible();
    await expect(page.locator('[data-test-try-again]')).toHaveCount(0);
  });

  test('shows an error page if versions fail to load', async ({ page, mirage, ember }) => {
    await mirage.addHook(server => {
      let crate = server.create('crate', { name: 'foo' });
      server.create('version', { crate, num: '3.2.1' });

      server.get('/api/v1/crates/:crate_name/versions', {}, 500);
    });

    await page.goto('/crates/foo/range/^3');
    await expect(page).toHaveURL('/crates/foo/range/%5E3');
    await expect(page.locator('[data-test-404-page]')).toBeVisible();
    await expect(page.locator('[data-test-title]')).toHaveText('foo: Failed to load version data');
    await expect(page.locator('[data-test-go-back]')).toHaveCount(0);
    await expect(page.locator('[data-test-try-again]')).toBeVisible();
  });
});
