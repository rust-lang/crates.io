import { expect, test } from '@/e2e/helper';
import { loadFixtures } from '@crates-io/msw/fixtures';
import { http, HttpResponse } from 'msw';

const BLOG_FEED_URL = 'https://blog.rust-lang.org/tags/crates-io/feed.xml';

test.describe('Acceptance | front page', { tag: '@acceptance' }, () => {
  test.use({ locale: 'en' });
  test('visiting /', async ({ page, msw, percy, a11y }) => {
    await loadFixtures(msw.db);

    await page.goto('/');

    await expect(page).toHaveURL('/');
    await expect(page).toHaveTitle('crates.io: Rust Package Registry');

    await expect(page.locator('[data-test-install-cargo-link]')).toBeVisible();
    await expect(page.locator('[data-test-login-button]')).toBeVisible();

    await expect(page.locator('[data-test-total-downloads] [data-test-value]')).toHaveText('143,345');
    await expect(page.locator('[data-test-total-crates] [data-test-value]')).toHaveText('23');

    let newCrate = page.locator('[data-test-new-crates] [data-test-crate-link="0"]');
    await expect(newCrate.locator('[data-test-title]')).toHaveText('serde');
    await expect(newCrate.locator('[data-test-subtitle]')).toHaveText(
      'A generic serialization/deserialization framework',
    );
    await expect(newCrate.locator('[data-test-version]')).toHaveText('1.0.0');
    await expect(newCrate).toHaveAttribute('href', '/crates/serde');

    let mostDownloaded = page.locator('[data-test-most-downloaded] [data-test-crate-link="0"]');
    await expect(mostDownloaded.locator('[data-test-title]')).toHaveText('serde');
    await expect(mostDownloaded.locator('[data-test-downloads]')).toContainText('51K');
    await expect(mostDownloaded).toHaveAttribute('href', '/crates/serde');

    let justUpdated = page.locator('[data-test-just-updated] [data-test-crate-link="0"]');
    await expect(justUpdated.locator('[data-test-title]')).toHaveText('nanomsg');
    await expect(justUpdated.locator('[data-test-version]')).toHaveText('0.6.1');
    await expect(justUpdated).toHaveAttribute('href', '/crates/nanomsg/0.6.1');

    let blogPosts = page.locator('[data-test-blog-posts] [data-test-blog-post]');
    await expect(blogPosts).toHaveCount(3);
    await expect(blogPosts.first().locator('[data-test-title]')).toHaveText('crates.io: development update');
    await expect(blogPosts.first()).toHaveAttribute(
      'href',
      'https://blog.rust-lang.org/2025/07/11/crates-io-development-update/',
    );
    await expect(blogPosts.nth(1).locator('[data-test-inside-rust-badge]')).toBeVisible();

    await percy.snapshot();
    await expect(page).toMatchAriaSnapshot({ name: 'aria.yml' });
    await a11y.audit();
  });

  test('error handling', async ({ page, msw }) => {
    msw.worker.use(http.get('/api/v1/summary', () => HttpResponse.json({}, { status: 500 })));

    await page.goto('/');
    await expect(page.locator('[data-test-lists]')).toHaveCount(0);
    await expect(page.locator('[data-test-error-message]')).toBeVisible();
    await expect(page.locator('[data-test-try-again-button]')).toBeEnabled();

    await msw.worker.resetHandlers();

    let deferred = Promise.withResolvers<void>();
    msw.worker.use(http.get('/api/v1/summary', () => deferred.promise));

    let button = page.locator('[data-test-try-again-button]');
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

  test('blog posts error handling', async ({ page, msw }) => {
    msw.worker.use(http.get(BLOG_FEED_URL, () => HttpResponse.text('', { status: 500 })));

    await page.goto('/');
    await expect(page.locator('[data-test-blog-posts-error]')).toBeVisible();
    await expect(page.locator('[data-test-lists]')).toBeVisible();
  });

  test('blog posts load independently of the summary', async ({ page, msw }) => {
    let summary = Promise.withResolvers<Response>();
    msw.worker.use(http.get('/api/v1/summary', () => summary.promise));

    await page.goto('/');
    await expect(page.locator('[data-test-blog-post]')).toHaveCount(3);
    await expect(page.locator('[data-test-lists]')).toBeVisible();
    await expect(page.locator('[data-test-crate-link]')).toHaveCount(0);

    summary.resolve(HttpResponse.json({}, { status: 500 }));

    await expect(page.locator('[data-test-error-message]')).toBeVisible();
    await expect(page.locator('[data-test-blog-post]')).toHaveCount(3);
  });

  test('summary loads independently of the blog posts', async ({ page, msw }) => {
    await loadFixtures(msw.db);
    msw.worker.use(http.get(BLOG_FEED_URL, () => new Promise(() => {})));

    await page.goto('/');
    await expect(page.locator('[data-test-new-crates] [data-test-crate-link="0"]')).toBeVisible();
    await expect(page.locator('[data-test-blog-post-placeholder]')).toHaveCount(3);
  });
});
