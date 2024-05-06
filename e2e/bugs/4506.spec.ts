import { test, expect } from '@/e2e/helper';

test.describe('Bug #4506', { tag: '@bugs' }, () => {
  test.beforeEach(async ({ mirage }) => {
    await mirage.addHook(server => {
      server.create('keyword', { keyword: 'no-std' });

      let foo = server.create('crate', { name: 'foo', keywordIds: ['no-std'] });
      server.create('version', { crate: foo });

      let bar = server.create('crate', { name: 'bar', keywordIds: ['no-std'] });
      server.create('version', { crate: bar });
    });
  });

  test('is fixed', async ({ page }) => {
    await page.goto('/crates/foo');
    await expect(page.locator('[data-test-keyword]')).toHaveCount(1);

    await page.click('[data-test-keyword="no-std"]');
    await expect(page.locator('[data-test-total-rows]')).toHaveText('2');
    await expect(page.locator('[data-test-crate-row]')).toHaveCount(2);
  });

  test('is fixed for /keywords too', async ({ page }) => {
    await page.goto('/keywords');
    await expect(page.locator('[data-test-keyword]')).toHaveCount(1);
    await expect(page.locator('[data-test-keyword="no-std"] [data-test-count]')).toHaveText('2 crates');

    await page.click('[data-test-keyword="no-std"] a');
    await expect(page.locator('[data-test-total-rows]')).toHaveText('2');
    await expect(page.locator('[data-test-crate-row]')).toHaveCount(2);
  });
});
