import { test, expect } from '@/e2e/helper';

test.describe('Acceptance | search', { tag: '@acceptance' }, () => {
  test('searching for "rust"', async ({ page, mirage, percy, a11y }) => {
    await mirage.addHook(server => {
      server.loadFixtures();
    });

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

  test('searching for "rust" from query', async ({ page, mirage }) => {
    await mirage.addHook(server => {
      server.loadFixtures();
    });

    await page.goto('/search?q=rust');

    await expect(page).toHaveURL('/search?q=rust');
    await expect(page).toHaveTitle("Search Results for 'rust' - crates.io: Rust Package Registry");

    await expect(page.locator('[data-test-search-input]')).toHaveValue('rust');
    await expect(page.locator('[data-test-header]')).toHaveText("Search Results for 'rust'");
    await expect(page.locator('[data-test-search-nav]')).toHaveText('Displaying 1-7 of 7 total results');
  });

  test('clearing search results', async ({ page, mirage }) => {
    await mirage.addHook(server => {
      server.loadFixtures();
    });

    await page.goto('/search?q=rust');

    await expect(page).toHaveURL('/search?q=rust');
    await expect(page.locator('[data-test-search-input]')).toHaveValue('rust');

    // favor navigation via link click over page.goto
    await page.getByRole('link', { name: 'crates.io', exact: true }).click();

    await expect(page).toHaveURL('/');
    await expect(page.locator('[data-test-search-input]')).toHaveValue('');
  });

  test('pressing S key to focus the search bar', async ({ page, mirage }) => {
    await mirage.addHook(server => {
      server.loadFixtures();
    });

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

  test('check search results are by default displayed by relevance', async ({ page, mirage }) => {
    await mirage.addHook(server => {
      server.loadFixtures();
    });

    await page.goto('/');
    await page.fill('[data-test-search-input]', 'rust');
    await page.locator('[data-test-search-form]').getByRole('button', { name: 'Submit' }).click();

    await expect(page.locator('[data-test-search-sort] [data-test-current-order]')).toHaveText('Relevance');
  });

  test('error handling when searching from the frontpage', async ({ page, mirage }) => {
    await mirage.addHook(server => {
      globalThis._routes = server._config.routes;
      let crate = server.create('crate', { name: 'rust' });
      server.create('version', { crate, num: '1.0.0' });

      server.get('/api/v1/crates', {}, 500);
    });

    await page.goto('/');
    await page.fill('[data-test-search-input]', 'rust');
    await page.locator('[data-test-search-form]').getByRole('button', { name: 'Submit' }).click();
    await expect(page.locator('[data-test-crate-row]')).toHaveCount(0);
    await expect(page.locator('[data-test-error-message]')).toBeVisible();
    await expect(page.locator('[data-test-try-again-button]')).toBeEnabled();

    await page.evaluate(() => {
      const deferred = (globalThis.deferred = require('rsvp').defer());
      server.get('/api/v1/crates', () => deferred.promise);
    });

    await page.click('[data-test-try-again-button]');
    await expect(page.locator('[data-test-page-header] [data-test-spinner]')).toBeVisible();
    await expect(page.locator('[data-test-crate-row]')).toHaveCount(0);
    await expect(page.locator('[data-test-error-message]')).toBeVisible();
    await expect(page.locator('[data-test-try-again-button]')).toBeDisabled();

    await page.evaluate(async () => {
      // Restore the routes
      globalThis._routes.call(server);
      const data = await globalThis.fetch('/api/v1/crates').then(r => r.json());
      globalThis.deferred.resolve(data);
    });
    await expect(page.locator('[data-test-error-message]')).toHaveCount(0);
    await expect(page.locator('[data-test-try-again-button]')).toHaveCount(0);
    await expect(page.locator('[data-test-crate-row]')).toHaveCount(1);
  });

  test('error handling when searching from the search page', async ({ page, mirage }) => {
    await mirage.addHook(server => {
      globalThis._routes = server._config.routes;
      let crate = server.create('crate', { name: 'rust' });
      server.create('version', { crate, num: '1.0.0' });
    });

    await page.goto('/search?q=rust');
    await expect(page.locator('[data-test-crate-row]')).toHaveCount(1);
    await expect(page.locator('[data-test-error-message]')).toHaveCount(0);
    await expect(page.locator('[data-test-try-again-button]')).toHaveCount(0);

    await page.evaluate(() => {
      server.get('/api/v1/crates', {}, 500);
    });

    await page.fill('[data-test-search-input]', 'ru');
    await page.locator('[data-test-search-form]').getByRole('button', { name: 'Submit' }).click();
    await expect(page.locator('[data-test-crate-row]')).toHaveCount(0);
    await expect(page.locator('[data-test-error-message]')).toBeVisible();
    await expect(page.locator('[data-test-try-again-button]')).toBeEnabled();

    await page.evaluate(() => {
      const deferred = (globalThis.deferred = require('rsvp').defer());
      server.get('/api/v1/crates', () => deferred.promise);
    });

    await page.click('[data-test-try-again-button]');
    await expect(page.locator('[data-test-page-header] [data-test-spinner]')).toBeVisible();
    await expect(page.locator('[data-test-crate-row]')).toHaveCount(0);
    await expect(page.locator('[data-test-error-message]')).toBeVisible();
    await expect(page.locator('[data-test-try-again-button]')).toBeDisabled();

    await page.evaluate(async () => {
      // Restore the routes
      globalThis._routes.call(server);
      const data = await globalThis.fetch('/api/v1/crates').then(r => r.json());
      globalThis.deferred.resolve(data);
    });
    await expect(page.locator('[data-test-crate-row]')).toHaveCount(1);
  });

  test('passes query parameters to the backend', async ({ page, mirage }) => {
    await mirage.config({ trackRequests: true });
    await mirage.addHook(server => {
      server.get('/api/v1/crates', () => ({ crates: [], meta: { total: 0 } }));
    });

    await page.goto('/search?q=rust&page=3&per_page=15&sort=new&all_keywords=fire ball');
    const queryParams = await page.evaluate(
      () => server.pretender.handledRequests.find(req => req.url.startsWith('/api/v1/crates')).queryParams,
    );
    expect(queryParams).toEqual({
      all_keywords: 'fire ball',
      page: '3',
      per_page: '15',
      q: 'rust',
      sort: 'new',
    });
  });

  test('supports `keyword:bla` filters', async ({ page, mirage }) => {
    await mirage.config({ trackRequests: true });
    await mirage.addHook(server => {
      server.get('/api/v1/crates', () => ({ crates: [], meta: { total: 0 } }));
    });

    await page.goto('/search?q=rust keyword:fire keyword:ball&page=3&per_page=15&sort=new');
    const queryParams = await page.evaluate(
      () => server.pretender.handledRequests.find(req => req.url.startsWith('/api/v1/crates')).queryParams,
    );
    expect(queryParams).toEqual({
      all_keywords: 'fire ball',
      page: '3',
      per_page: '15',
      q: 'rust',
      sort: 'new',
    });
  });

  test('`all_keywords` query parameter takes precedence over `keyword` filters', async ({ page, mirage }) => {
    await mirage.config({ trackRequests: true });
    await mirage.addHook(server => {
      server.get('/api/v1/crates', () => ({ crates: [], meta: { total: 0 } }));
    });

    await page.goto('/search?q=rust keywords:foo&page=3&per_page=15&sort=new&all_keywords=fire ball');
    const queryParams = await page.evaluate(
      () => server.pretender.handledRequests.find(req => req.url.startsWith('/api/v1/crates')).queryParams,
    );
    expect(queryParams).toEqual({
      all_keywords: 'fire ball',
      page: '3',
      per_page: '15',
      q: 'rust keywords:foo',
      sort: 'new',
    });
  });

  test('visiting without query parameters works', async ({ page, mirage }) => {
    await mirage.addHook(server => {
      server.loadFixtures();
    });

    await page.goto('/search');

    await expect(page).toHaveURL('/search');
    await expect(page).toHaveTitle('Search Results - crates.io: Rust Package Registry');

    await expect(page.locator('[data-test-header]')).toHaveText('Search Results');
    await expect(page.locator('[data-test-search-nav]')).toHaveText('Displaying 1-10 of 23 total results');
    await expect(page.locator('[data-test-crate-row="0"] [data-test-crate-link]')).toHaveText('kinetic-rust');
    await expect(page.locator('[data-test-crate-row="0"] [data-test-version]')).toHaveText('v0.0.16');
  });
});
