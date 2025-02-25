import { expect, test } from '@/e2e/helper';
import { loadFixtures } from '@crates-io/msw/fixtures';
import { http, HttpResponse } from 'msw';

test.describe('Acceptance | crates page', { tag: '@acceptance' }, () => {
  // should match the default set in the crates controller
  const per_page = 50;

  test('visiting the crates page from the front page', async ({ page, msw, percy, a11y }) => {
    loadFixtures(msw.db);

    await page.goto('/');
    await page.click('[data-test-all-crates-link]');

    await expect(page).toHaveURL('/crates');
    await expect(page).toHaveTitle('Crates - crates.io: Rust Package Registry');

    await percy.snapshot();
    await a11y.audit();
  });

  test('visiting the crates page directly', async ({ page, msw }) => {
    loadFixtures(msw.db);

    await page.goto('/crates');
    await page.click('[data-test-all-crates-link]');

    await expect(page).toHaveURL('/crates');
    await expect(page).toHaveTitle('Crates - crates.io: Rust Package Registry');
  });

  test('listing crates', async ({ page, msw }) => {
    const per_page = 50;
    for (let i = 1; i <= per_page; i++) {
      let crate = msw.db.crate.create();
      msw.db.version.create({ crate });
    }

    await page.goto('/crates');

    await expect(page.locator('[data-test-crates-nav] [data-test-current-rows]')).toHaveText(`1-${per_page}`);
    await expect(page.locator('[data-test-crates-nav] [data-test-total-rows]')).toHaveText(`${per_page}`);
  });

  test('navigating to next page of crates', async ({ page, msw }) => {
    const per_page = 50;
    for (let i = 1; i <= per_page + 2; i++) {
      let crate = msw.db.crate.create();
      msw.db.version.create({ crate });
    }
    const page_start = per_page + 1;
    const total = per_page + 2;

    await page.goto('/crates');
    await page.click('[data-test-pagination-next]');

    await expect(page).toHaveURL('/crates?page=2');
    await expect(page.locator('[data-test-crates-nav] [data-test-current-rows]')).toHaveText(`${page_start}-${total}`);
    await expect(page.locator('[data-test-crates-nav] [data-test-total-rows]')).toHaveText(`${total}`);
  });

  test('crates default sort is alphabetical', async ({ page, msw }) => {
    loadFixtures(msw.db);

    await page.goto('/crates');

    await expect(page.locator('[data-test-crates-sort] [data-test-current-order]')).toHaveText('Recent Downloads');
  });

  test('downloads appears for each crate on crate list', async ({ page, msw }) => {
    loadFixtures(msw.db);

    await page.goto('/crates');
    await expect(page.locator('[data-test-crate-row="0"] [data-test-downloads]')).toHaveText('All-Time: 21,573');
  });

  test('recent downloads appears for each crate on crate list', async ({ page, msw }) => {
    loadFixtures(msw.db);

    await page.goto('/crates');
    await expect(page.locator('[data-test-crate-row="0"] [data-test-recent-downloads]')).toHaveText('Recent: 2,000');
  });

  test('shows error message screen', async ({ page, msw }) => {
    loadFixtures(msw.db);

    let detail =
      'Page 1 is unavailable for performance reasons. Please take a look at https://crates.io/data-access for alternatives.';
    let error = HttpResponse.json({ errors: [{ detail }] }, { status: 400 });
    await msw.worker.use(http.get('/api/v1/crates', () => error));

    await page.goto('/crates');
    await expect(page.locator('[data-test-404-page]')).toBeVisible();
    await expect(page.locator('[data-test-title]')).toHaveText('Failed to load crate list');
    await expect(page.locator('[data-test-details]')).toHaveText(detail);
    await expect(page.locator('[data-test-try-again]')).toBeVisible();
    await expect(page.locator('[data-test-go-back]')).not.toBeVisible();

    await msw.worker.resetHandlers();
    await page.click('[data-test-try-again]');
    await expect(page.locator('[data-test-404-page]')).not.toBeVisible();
    await expect(page.locator('[data-test-crate-row]')).toHaveCount(23);
  });
});
