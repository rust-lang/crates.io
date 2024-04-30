import { AxeBuilder } from '@axe-core/playwright';
import { percySnapshot, test, expect } from '@/e2e/helper';
import axeConfig from '@/tests/axe-config';

test.describe('Acceptance | categories', { tag: '@acceptance' }, () => {
  test('listing categories', async ({ page, mirage }, testInfo) => {
    await mirage.addHook(server => {
      server.create('category', { category: 'API bindings' });
      server.create('category', { category: 'Algorithms' });
      server.createList('crate', 1, { categoryIds: ['algorithms'] });
      server.create('category', { category: 'Asynchronous' });
      server.createList('crate', 15, { categoryIds: ['asynchronous'] });
      server.create('category', { category: 'Everything', crates_cnt: 1234 });
    });

    await page.goto('/categories');

    await expect(page.locator('[data-test-category="api-bindings"] [data-test-crate-count]')).toHaveText('0 crates');
    await expect(page.locator('[data-test-category="algorithms"] [data-test-crate-count]')).toHaveText('1 crate');
    await expect(page.locator('[data-test-category="asynchronous"] [data-test-crate-count]')).toHaveText('15 crates');
    await expect(page.locator('[data-test-category="everything"] [data-test-crate-count]')).toHaveText('1,234 crates');

    await percySnapshot(page, testInfo);
    const a11yAuditResult = await new AxeBuilder({ page }).options(axeConfig).analyze();
    expect(a11yAuditResult.violations).toEqual([]);
  });

  test('category/:category_id index default sort is recent-downloads', async ({ page, mirage }, testInfo) => {
    await mirage.addHook(server => {
      server.create('category', { category: 'Algorithms' });
    });
    await page.goto('/categories/algorithms');

    await expect(page.locator('[data-test-category-sort] [data-test-current-order]')).toHaveText('Recent Downloads');

    await percySnapshot(page, testInfo);
    const a11yAuditResult = await new AxeBuilder({ page }).options(axeConfig).analyze();
    expect(a11yAuditResult.violations).toEqual([]);
  });

  test('listing category slugs', async ({ page, mirage }) => {
    await mirage.addHook(server => {
      server.create('category', { category: 'Algorithms', description: 'Crates for algorithms' });
      server.create('category', { category: 'Asynchronous', description: 'Async crates' });
    });
    await page.goto('/category_slugs');

    await expect(page.locator('[data-test-category-slug="algorithms"]')).toHaveText('algorithms');
    await expect(page.locator('[data-test-category-description="algorithms"]')).toHaveText('Crates for algorithms');
    await expect(page.locator('[data-test-category-slug="asynchronous"]')).toHaveText('asynchronous');
    await expect(page.locator('[data-test-category-description="asynchronous"]')).toHaveText('Async crates');
  });
});

test.describe('Acceptance | categories (locale: de)', { tag: '@acceptance' }, () => {
  test.use({ locale: 'de' });
  test('listing categories', async ({ page, mirage }) => {
    await mirage.addHook(server => {
      server.create('category', { category: 'Everything', crates_cnt: 1234 });
    });
    await page.goto('categories');

    await expect(page.locator('[data-test-category="everything"] [data-test-crate-count]')).toHaveText('1.234 crates');
  });
});
