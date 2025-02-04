import { expect, test } from '@/e2e/helper';
import { http, HttpResponse } from 'msw';

test.describe('Route | crate.version | docs link', { tag: '@routes' }, () => {
  test('shows regular documentation link', async ({ page, msw }) => {
    let crate = msw.db.crate.create({ name: 'foo', documentation: 'https://foo.io/docs' });
    msw.db.version.create({ crate, num: '1.0.0' });

    await page.goto('/crates/foo');
    await expect(page.locator('[data-test-docs-link] a')).toHaveAttribute('href', 'https://foo.io/docs');
  });

  test('show no docs link if `documentation` is unspecified and there are no related docs.rs builds', async ({
    page,
    msw,
  }) => {
    let crate = msw.db.crate.create({ name: 'foo' });
    msw.db.version.create({ crate, num: '1.0.0' });

    let error = HttpResponse.text('not found', { status: 404 });
    msw.worker.use(http.get('https://docs.rs/crate/:crate/:version/status.json', () => error));

    await page.goto('/crates/foo');
    await expect(page.getByRole('link', { name: 'crates.io', exact: true })).toHaveCount(1);

    await expect(page.locator('[data-test-docs-link] a')).toHaveCount(0);
  });

  test('show docs link if `documentation` is unspecified and there are related docs.rs builds', async ({
    page,
    msw,
  }) => {
    let crate = msw.db.crate.create({ name: 'foo' });
    msw.db.version.create({ crate, num: '1.0.0' });

    let response = HttpResponse.json({
      doc_status: true,
      version: '1.0.0',
    });
    msw.worker.use(http.get('https://docs.rs/crate/:crate/:version/status.json', () => response));

    await page.goto('/crates/foo');
    await expect(page.locator('[data-test-docs-link] a')).toHaveAttribute('href', 'https://docs.rs/foo/1.0.0');
  });

  test('show original docs link if `documentation` points to docs.rs and there are no related docs.rs builds', async ({
    page,
    msw,
  }) => {
    let crate = msw.db.crate.create({ name: 'foo', documentation: 'https://docs.rs/foo/0.6.2' });
    msw.db.version.create({ crate, num: '1.0.0' });

    let error = HttpResponse.text('not found', { status: 404 });
    msw.worker.use(http.get('https://docs.rs/crate/:crate/:version/status.json', () => error));

    await page.goto('/crates/foo');
    await expect(page.locator('[data-test-docs-link] a')).toHaveAttribute('href', 'https://docs.rs/foo/0.6.2');
  });

  test('show updated docs link if `documentation` points to docs.rs and there are related docs.rs builds', async ({
    page,
    msw,
  }) => {
    let crate = msw.db.crate.create({ name: 'foo', documentation: 'https://docs.rs/foo/0.6.2' });
    msw.db.version.create({ crate, num: '1.0.0' });

    let response = HttpResponse.json({
      doc_status: true,
      version: '1.0.0',
    });
    msw.worker.use(http.get('https://docs.rs/crate/:crate/:version/status.json', () => response));

    await page.goto('/crates/foo');
    await expect(page.locator('[data-test-docs-link] a')).toHaveAttribute('href', 'https://docs.rs/foo/1.0.0');
  });

  test('ajax errors are ignored', async ({ page, msw }) => {
    let crate = msw.db.crate.create({ name: 'foo', documentation: 'https://docs.rs/foo/0.6.2' });
    msw.db.version.create({ crate, num: '1.0.0' });

    let error = HttpResponse.text('error', { status: 500 });
    msw.worker.use(http.get('https://docs.rs/crate/:crate/:version/status.json', () => error));

    await page.goto('/crates/foo');
    await expect(page.locator('[data-test-docs-link] a')).toHaveAttribute('href', 'https://docs.rs/foo/0.6.2');
  });

  test('empty docs.rs responses are ignored', async ({ page, msw }) => {
    let crate = msw.db.crate.create({ name: 'foo', documentation: 'https://docs.rs/foo/0.6.2' });
    msw.db.version.create({ crate, num: '0.6.2' });

    let response = HttpResponse.json({});
    msw.worker.use(http.get('https://docs.rs/crate/:crate/:version/status.json', () => response));

    await page.goto('/crates/foo');
    await expect(page.locator('[data-test-docs-link] a')).toHaveAttribute('href', 'https://docs.rs/foo/0.6.2');
  });
});
