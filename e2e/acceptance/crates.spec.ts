import { test, expect } from '@/e2e/helper';

test.describe('Acceptance | crates page', { tag: '@acceptance' }, () => {
  // should match the default set in the crates controller
  const per_page = 50;

  test('visiting the crates page from the front page', async ({ page, mirage, percy, a11y }) => {
    await mirage.addHook(server => {
      server.loadFixtures();
    });

    await page.goto('/');
    await page.click('[data-test-all-crates-link]');

    await expect(page).toHaveURL('/crates');
    await expect(page).toHaveTitle('Crates - crates.io: Rust Package Registry');

    await percy.snapshot();
    await a11y.audit();
  });

  test('visiting the crates page directly', async ({ page, mirage }) => {
    await mirage.addHook(server => {
      server.loadFixtures();
    });

    await page.goto('/crates');
    await page.click('[data-test-all-crates-link]');

    await expect(page).toHaveURL('/crates');
    await expect(page).toHaveTitle('Crates - crates.io: Rust Package Registry');
  });

  test('listing crates', async ({ page, mirage }) => {
    await mirage.addHook(server => {
      const per_page = 50;
      for (let i = 1; i <= per_page; i++) {
        let crate = server.create('crate');
        server.create('version', { crate });
      }
    });

    await page.goto('/crates');

    await expect(page.locator('[data-test-crates-nav] [data-test-current-rows]')).toHaveText(`1-${per_page}`);
    await expect(page.locator('[data-test-crates-nav] [data-test-total-rows]')).toHaveText(`${per_page}`);
  });

  test('navigating to next page of crates', async ({ page, mirage }) => {
    await mirage.addHook(server => {
      const per_page = 50;
      for (let i = 1; i <= per_page + 2; i++) {
        let crate = server.create('crate');
        server.create('version', { crate });
      }
    });
    const page_start = per_page + 1;
    const total = per_page + 2;

    await page.goto('/crates');
    await page.click('[data-test-pagination-next]');

    await expect(page).toHaveURL('/crates?page=2');
    await expect(page.locator('[data-test-crates-nav] [data-test-current-rows]')).toHaveText(`${page_start}-${total}`);
    await expect(page.locator('[data-test-crates-nav] [data-test-total-rows]')).toHaveText(`${total}`);
  });

  test('crates default sort is alphabetical', async ({ page, mirage }) => {
    await mirage.addHook(server => {
      server.loadFixtures();
    });

    await page.goto('/crates');

    await expect(page.locator('[data-test-crates-sort] [data-test-current-order]')).toHaveText('Alphabetical');
  });

  test('downloads appears for each crate on crate list', async ({ page, mirage }) => {
    await mirage.addHook(server => {
      server.loadFixtures();
    });

    await page.goto('/crates');
    await expect(page.locator('[data-test-crate-row="0"] [data-test-downloads]')).toHaveText('All-Time: 497');
  });

  test('recent downloads appears for each crate on crate list', async ({ page, mirage }) => {
    await mirage.addHook(server => {
      server.loadFixtures();
    });

    await page.goto('/crates');
    await expect(page.locator('[data-test-crate-row="0"] [data-test-recent-downloads]')).toHaveText('Recent: 497');
  });
});
