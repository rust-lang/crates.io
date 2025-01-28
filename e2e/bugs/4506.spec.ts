import { expect, test } from '@/e2e/helper';

test.describe('Bug #4506', { tag: '@bugs' }, () => {
  test.beforeEach(async ({ msw }) => {
    let noStd = msw.db.keyword.create({ keyword: 'no-std' });

    let foo = msw.db.crate.create({ name: 'foo', keywords: [noStd] });
    msw.db.version.create({ crate: foo });

    let bar = msw.db.crate.create({ name: 'bar', keywords: [noStd] });
    msw.db.version.create({ crate: bar });
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
