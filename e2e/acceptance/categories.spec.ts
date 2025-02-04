import { expect, test } from '@/e2e/helper';

test.describe('Acceptance | categories', { tag: '@acceptance' }, () => {
  test('listing categories', async ({ page, msw, percy, a11y }) => {
    msw.db.category.create({ category: 'API bindings' });
    let algos = msw.db.category.create({ category: 'Algorithms' });
    msw.db.crate.create({ categories: [algos] });
    let async = msw.db.category.create({ category: 'Asynchronous' });
    Array.from({ length: 15 }).forEach(() => msw.db.crate.create({ categories: [async] }));
    msw.db.category.create({ category: 'Everything', crates_cnt: 1234 });

    await page.goto('/categories');

    await expect(page.locator('[data-test-category="api-bindings"] [data-test-crate-count]')).toHaveText('0 crates');
    await expect(page.locator('[data-test-category="algorithms"] [data-test-crate-count]')).toHaveText('1 crate');
    await expect(page.locator('[data-test-category="asynchronous"] [data-test-crate-count]')).toHaveText('15 crates');
    await expect(page.locator('[data-test-category="everything"] [data-test-crate-count]')).toHaveText('1,234 crates');

    await percy.snapshot();
    await a11y.audit();
  });

  test('category/:category_id index default sort is recent-downloads', async ({ page, msw, percy, a11y }) => {
    msw.db.category.create({ category: 'Algorithms' });
    await page.goto('/categories/algorithms');

    await expect(page.locator('[data-test-category-sort] [data-test-current-order]')).toHaveText('Recent Downloads');

    await percy.snapshot();
    await a11y.audit();
  });

  test('listing category slugs', async ({ page, msw }) => {
    msw.db.category.create({ category: 'Algorithms', description: 'Crates for algorithms' });
    msw.db.category.create({ category: 'Asynchronous', description: 'Async crates' });
    await page.goto('/category_slugs');

    await expect(page.locator('[data-test-category-slug="algorithms"]')).toHaveText('algorithms');
    await expect(page.locator('[data-test-category-description="algorithms"]')).toHaveText('Crates for algorithms');
    await expect(page.locator('[data-test-category-slug="asynchronous"]')).toHaveText('asynchronous');
    await expect(page.locator('[data-test-category-description="asynchronous"]')).toHaveText('Async crates');
  });
});

test.describe('Acceptance | categories (locale: de)', { tag: '@acceptance' }, () => {
  test.use({ locale: 'de' });
  test('listing categories', async ({ page, msw }) => {
    msw.db.category.create({ category: 'Everything', crates_cnt: 1234 });
    await page.goto('categories');

    await expect(page.locator('[data-test-category="everything"] [data-test-crate-count]')).toHaveText('1.234 crates');
  });
});
