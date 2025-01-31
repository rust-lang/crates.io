import { defer } from '@/e2e/deferred';
import { expect, test } from '@/e2e/helper';
import { loadFixtures } from '@crates-io/msw/fixtures';
import { http, HttpResponse } from 'msw';

test.describe('Acceptance | search', { tag: '@acceptance' }, () => {
  test('searching for "rust"', async ({ page, msw, percy, a11y }) => {
    loadFixtures(msw.db);

    await page.goto('/');
    await page.fill('[data-test-search-input]', 'rust');
    await page.locator('[data-test-search-form]').getByRole('button', { name: 'Submit' }).click();

    await expect(page).toHaveURL('/search?q=rust');
    await expect(page).toHaveTitle("Search Results for 'rust' - crates.io: Rust Package Registry");

    await expect(page.locator('[data-test-header]')).toHaveText("Search Results for 'rust'");
    await expect(page.locator('[data-test-search-nav]')).toHaveText('Displaying 1-7 of 7 total results');
    await expect(page.locator('[data-test-search-sort]')).toHaveText(
      'Sort by Relevance Relevance All-Time Downloads Recent Downloads Recent Updates Newly Added',
    );
    await expect(page.locator('[data-test-crate-row="0"] [data-test-crate-link]')).toHaveText('kinetic-rust');
    await expect(page.locator('[data-test-crate-row="0"] [data-test-version]')).toHaveText('v0.0.16');

    await expect(page.locator('[data-test-crate-row="0"] [data-test-description]')).toHaveText(
      'A Kinetic protocol library written in Rust',
    );
    await expect(page.locator('[data-test-crate-row="0"] [data-test-downloads]')).toHaveText('All-Time: 225');
    await expect(page.locator('[data-test-crate-row="0"] [data-test-updated-at]')).toBeVisible();

    await percy.snapshot();
    await a11y.audit();
  });

  test('searching for "rust" from query', async ({ page, msw }) => {
    loadFixtures(msw.db);

    await page.goto('/search?q=rust');

    await expect(page).toHaveURL('/search?q=rust');
    await expect(page).toHaveTitle("Search Results for 'rust' - crates.io: Rust Package Registry");

    await expect(page.locator('[data-test-search-input]')).toHaveValue('rust');
    await expect(page.locator('[data-test-header]')).toHaveText("Search Results for 'rust'");
    await expect(page.locator('[data-test-search-nav]')).toHaveText('Displaying 1-7 of 7 total results');
  });

  test('clearing search results', async ({ page, msw }) => {
    loadFixtures(msw.db);

    await page.goto('/search?q=rust');

    await expect(page).toHaveURL('/search?q=rust');
    await expect(page.locator('[data-test-search-input]')).toHaveValue('rust');

    // favor navigation via link click over page.goto
    await page.getByRole('link', { name: 'crates.io', exact: true }).click();

    await expect(page).toHaveURL('/');
    await expect(page.locator('[data-test-search-input]')).toHaveValue('');
  });

  test('pressing S key to focus the search bar', async ({ page, msw }) => {
    loadFixtures(msw.db);

    await page.goto('/');

    const searchInput = page.locator('[data-test-search-input]');
    await searchInput.blur();
    await page.keyboard.press('a');
    await expect(searchInput).not.toBeFocused();

    await searchInput.blur();
    await page.keyboard.press('s');
    await expect(page.locator('[data-test-search-input]')).toBeFocused();

    await searchInput.blur();
    await page.keyboard.press('s');
    await expect(page.locator('[data-test-search-input]')).toBeFocused();

    await searchInput.blur();
    await page.keyboard.press('S');
    await expect(page.locator('[data-test-search-input]')).toBeFocused();

    await searchInput.blur();
    await page.keyboard.down('Shift');
    await page.keyboard.press('s');
    await page.keyboard.up('Shift');
    await expect(page.locator('[data-test-search-input]')).toBeFocused();
  });

  test('check search results are by default displayed by relevance', async ({ page, msw }) => {
    loadFixtures(msw.db);

    await page.goto('/');
    await page.fill('[data-test-search-input]', 'rust');
    await page.locator('[data-test-search-form]').getByRole('button', { name: 'Submit' }).click();

    await expect(page.locator('[data-test-search-sort] [data-test-current-order]')).toHaveText('Relevance');
  });

  test('error handling when searching from the frontpage', async ({ page, msw }) => {
    let crate = msw.db.crate.create({ name: 'rust' });
    msw.db.version.create({ crate, num: '1.0.0' });

    let error = HttpResponse.json({}, { status: 500 });
    await msw.worker.use(http.get('/api/v1/crates', () => error));

    await page.goto('/');
    await page.fill('[data-test-search-input]', 'rust');
    await page.locator('[data-test-search-form]').getByRole('button', { name: 'Submit' }).click();
    await expect(page.locator('[data-test-crate-row]')).toHaveCount(0);
    await expect(page.locator('[data-test-error-message]')).toBeVisible();
    await expect(page.locator('[data-test-try-again-button]')).toBeEnabled();

    await msw.worker.resetHandlers();
    let deferred = defer();
    await msw.worker.use(http.get('/api/v1/crates', () => deferred.promise));

    await page.click('[data-test-try-again-button]');
    await expect(page.locator('[data-test-page-header] [data-test-spinner]')).toBeVisible();
    await expect(page.locator('[data-test-crate-row]')).toHaveCount(0);
    await expect(page.locator('[data-test-error-message]')).toBeVisible();
    await expect(page.locator('[data-test-try-again-button]')).toBeDisabled();

    deferred.resolve();
    await expect(page.locator('[data-test-error-message]')).toHaveCount(0);
    await expect(page.locator('[data-test-try-again-button]')).toHaveCount(0);
    await expect(page.locator('[data-test-crate-row]')).toHaveCount(1);
  });

  test('error handling when searching from the search page', async ({ page, msw }) => {
    let crate = msw.db.crate.create({ name: 'rust' });
    msw.db.version.create({ crate, num: '1.0.0' });

    await page.goto('/search?q=rust');
    await expect(page.locator('[data-test-crate-row]')).toHaveCount(1);
    await expect(page.locator('[data-test-error-message]')).toHaveCount(0);
    await expect(page.locator('[data-test-try-again-button]')).toHaveCount(0);

    let error = HttpResponse.json({}, { status: 500 });
    await msw.worker.use(http.get('/api/v1/crates', () => error));

    await page.fill('[data-test-search-input]', 'ru');
    await page.locator('[data-test-search-form]').getByRole('button', { name: 'Submit' }).click();
    await expect(page.locator('[data-test-crate-row]')).toHaveCount(0);
    await expect(page.locator('[data-test-error-message]')).toBeVisible();
    await expect(page.locator('[data-test-try-again-button]')).toBeEnabled();

    await msw.worker.resetHandlers();
    let deferred = defer();
    await msw.worker.use(http.get('/api/v1/crates', () => deferred.promise));

    await page.click('[data-test-try-again-button]');
    await expect(page.locator('[data-test-page-header] [data-test-spinner]')).toBeVisible();
    await expect(page.locator('[data-test-crate-row]')).toHaveCount(0);
    await expect(page.locator('[data-test-error-message]')).toBeVisible();
    await expect(page.locator('[data-test-try-again-button]')).toBeDisabled();

    deferred.resolve();
    await expect(page.locator('[data-test-crate-row]')).toHaveCount(1);
  });

  test('passes query parameters to the backend', async ({ page, msw }) => {
    msw.worker.use(
      http.get('/api/v1/crates', function ({ request }) {
        let url = new URL(request.url);
        expect(Object.fromEntries(url.searchParams.entries())).toEqual({
          all_keywords: 'fire ball',
          page: '3',
          per_page: '15',
          q: 'rust',
          sort: 'new',
        });

        return HttpResponse.json({ crates: [], meta: { total: 0 } });
      }),
    );

    await page.goto('/search?q=rust&page=3&per_page=15&sort=new&all_keywords=fire ball');
  });

  test('supports `keyword:bla` filters', async ({ page, msw }) => {
    msw.worker.use(
      http.get('/api/v1/crates', function ({ request }) {
        let url = new URL(request.url);
        expect(Object.fromEntries(url.searchParams.entries())).toEqual({
          all_keywords: 'fire ball',
          page: '3',
          per_page: '15',
          q: 'rust',
          sort: 'new',
        });

        return HttpResponse.json({ crates: [], meta: { total: 0 } });
      }),
    );

    await page.goto('/search?q=rust keyword:fire keyword:ball&page=3&per_page=15&sort=new');
  });

  test('`all_keywords` query parameter takes precedence over `keyword` filters', async ({ page, msw }) => {
    msw.worker.use(
      http.get('/api/v1/crates', function ({ request }) {
        let url = new URL(request.url);
        expect(Object.fromEntries(url.searchParams.entries())).toEqual({
          all_keywords: 'fire ball',
          page: '3',
          per_page: '15',
          q: 'rust keywords:foo',
          sort: 'new',
        });

        return HttpResponse.json({ crates: [], meta: { total: 0 } });
      }),
    );

    await page.goto('/search?q=rust keywords:foo&page=3&per_page=15&sort=new&all_keywords=fire ball');
  });

  test('visiting without query parameters works', async ({ page, msw }) => {
    loadFixtures(msw.db);

    await page.goto('/search');

    await expect(page).toHaveURL('/search');
    await expect(page).toHaveTitle('Search Results - crates.io: Rust Package Registry');

    await expect(page.locator('[data-test-header]')).toHaveText('Search Results');
    await expect(page.locator('[data-test-search-nav]')).toHaveText('Displaying 1-10 of 23 total results');
    await expect(page.locator('[data-test-crate-row="0"] [data-test-crate-link]')).toHaveText('kinetic-rust');
    await expect(page.locator('[data-test-crate-row="0"] [data-test-version]')).toHaveText('v0.0.16');
  });
});
