import { defer } from '@/e2e/deferred';
import { expect, test } from '@/e2e/helper';
import { loadFixtures } from '@crates-io/msw/fixtures';
import { http, HttpResponse } from 'msw';

test.describe('Acceptance | front page', { tag: '@acceptance' }, () => {
  test.use({ locale: 'en' });
  test('visiting /', async ({ page, msw, percy, a11y }) => {
    loadFixtures(msw.db);

    await page.goto('/');

    await expect(page).toHaveURL('/');
    await expect(page).toHaveTitle('crates.io: Rust Package Registry');

    await expect(page.locator('[data-test-install-cargo-link]')).toBeVisible();
    await expect(page.locator('[data-test-all-crates-link]')).toBeVisible();
    await expect(page.locator('[data-test-login-button]')).toBeVisible();

    await expect(page.locator('[data-test-total-downloads] [data-test-value]')).toHaveText('143,345');
    await expect(page.locator('[data-test-total-crates] [data-test-value]')).toHaveText('23');

    await expect(page.locator('[data-test-new-crates] [data-test-crate-link="0"]')).toHaveText('serde v1.0.0');
    await expect(page.locator('[data-test-new-crates] [data-test-crate-link="0"]')).toHaveAttribute(
      'href',
      '/crates/serde',
    );

    await expect(page.locator('[data-test-most-downloaded] [data-test-crate-link="0"]')).toHaveText('serde');
    await expect(page.locator('[data-test-most-downloaded] [data-test-crate-link="0"]')).toHaveAttribute(
      'href',
      '/crates/serde',
    );

    await expect(page.locator('[data-test-just-updated] [data-test-crate-link="0"]')).toHaveText('nanomsg v0.6.1');
    await expect(page.locator('[data-test-just-updated] [data-test-crate-link="0"]')).toHaveAttribute(
      'href',
      '/crates/nanomsg/0.6.1',
    );

    await percy.snapshot();
    await a11y.audit();
  });

  test('error handling', async ({ page, msw }) => {
    await msw.worker.use(http.get('/api/v1/summary', () => HttpResponse.json({}, { status: 500 })));

    await page.goto('/');
    await expect(page.locator('[data-test-lists]')).toHaveCount(0);
    await expect(page.locator('[data-test-error-message]')).toBeVisible();
    await expect(page.locator('[data-test-try-again-button]')).toBeEnabled();

    await msw.worker.resetHandlers();

    let deferred = defer();
    msw.worker.use(http.get('/api/v1/summary', () => deferred.promise));

    const button = page.locator('[data-test-try-again-button]');
    await button.click();
    await expect(button.locator('[data-test-spinner]')).toBeVisible();
    await expect(page.locator('[data-test-lists]')).toHaveCount(0);
    await expect(page.locator('[data-test-error-message]')).toBeVisible();
    await expect(page.locator('[data-test-try-again-button]')).toBeDisabled();

    deferred.resolve();

    await expect(page.locator('[data-test-lists]')).toBeVisible();
    await expect(page.locator('[data-test-error-message]')).toHaveCount(0);
    await expect(page.locator('[data-test-try-again-button]')).toHaveCount(0);
  });
});
