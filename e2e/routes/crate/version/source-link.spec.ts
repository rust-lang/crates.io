import { expect, test } from '@/e2e/helper';
import { http, HttpResponse } from 'msw';

test.describe('Route | crate.version | source link', { tag: '@routes' }, () => {
  test('show docs.rs source link even if non-docs.rs documentation link is specified', async ({ page, msw }) => {
    let crate = await msw.db.crate.create({ name: 'foo', documentation: 'https://foo.io/docs' });
    await msw.db.version.create({ crate, num: '1.0.0' });

    let response = HttpResponse.json({
      doc_status: false,
      version: '1.0.0',
    });
    msw.worker.use(http.get('https://docs.rs/crate/:crate/:version/status.json', () => response));

    await page.goto('/crates/foo');
    await expect(page.locator('[data-test-source-link] a')).toHaveAttribute(
      'href',
      'https://docs.rs/crate/foo/1.0.0/source/',
    );
  });

  test('show no source link if there are no related docs.rs builds', async ({ page, msw }) => {
    let crate = await msw.db.crate.create({ name: 'foo' });
    await msw.db.version.create({ crate, num: '1.0.0' });

    let error = HttpResponse.text('not found', { status: 404 });
    msw.worker.use(http.get('https://docs.rs/crate/:crate/:version/status.json', () => error));

    await page.goto('/crates/foo');
    await expect(page.getByRole('link', { name: 'crates.io', exact: true })).toHaveCount(1);

    await expect(page.locator('[data-test-source-link] a')).toHaveCount(0);
  });

  test('show source link if `documentation` is unspecified and there are related docs.rs builds', async ({
    page,
    msw,
  }) => {
    let crate = await msw.db.crate.create({ name: 'foo' });
    await msw.db.version.create({ crate, num: '1.0.0' });

    let response = HttpResponse.json({
      doc_status: true,
      version: '1.0.0',
    });
    msw.worker.use(http.get('https://docs.rs/crate/:crate/:version/status.json', () => response));

    await page.goto('/crates/foo');
    await expect(page.locator('[data-test-source-link] a')).toHaveAttribute(
      'href',
      'https://docs.rs/crate/foo/1.0.0/source/',
    );
  });

  test('show no source link if `documentation` points to docs.rs and there are no related docs.rs builds', async ({
    page,
    msw,
  }) => {
    let crate = await msw.db.crate.create({ name: 'foo', documentation: 'https://docs.rs/foo/0.6.2' });
    await msw.db.version.create({ crate, num: '1.0.0' });

    let error = HttpResponse.text('not found', { status: 404 });
    msw.worker.use(http.get('https://docs.rs/crate/:crate/:version/status.json', () => error));

    await page.goto('/crates/foo');
    await expect(page.locator('[data-test-source-link] a')).toHaveCount(0);
  });

  test('show source link if `documentation` points to docs.rs and there are related docs.rs builds', async ({
    page,
    msw,
  }) => {
    let crate = await msw.db.crate.create({ name: 'foo', documentation: 'https://docs.rs/foo/0.6.2' });
    await msw.db.version.create({ crate, num: '1.0.0' });

    let response = HttpResponse.json({
      doc_status: true,
      version: '1.0.0',
    });
    msw.worker.use(http.get('https://docs.rs/crate/:crate/:version/status.json', () => response));

    await page.goto('/crates/foo');
    await expect(page.locator('[data-test-source-link] a')).toHaveAttribute(
      'href',
      'https://docs.rs/crate/foo/1.0.0/source/',
    );
  });

  test('ajax errors are ignored, but show no source link', async ({ page, msw }) => {
    let crate = await msw.db.crate.create({ name: 'foo', documentation: 'https://docs.rs/foo/0.6.2' });
    await msw.db.version.create({ crate, num: '1.0.0' });

    let error = HttpResponse.text('error', { status: 500 });
    msw.worker.use(http.get('https://docs.rs/crate/:crate/:version/status.json', () => error));

    await page.goto('/crates/foo');
    await expect(page.locator('[data-test-source-link] a')).toHaveCount(0);
  });

  test('empty docs.rs responses are ignored, still show source link', async ({ page, msw }) => {
    let crate = await msw.db.crate.create({ name: 'foo', documentation: 'https://docs.rs/foo/0.6.2' });
    await msw.db.version.create({ crate, num: '0.6.2' });

    let response = HttpResponse.json({});
    msw.worker.use(http.get('https://docs.rs/crate/:crate/:version/status.json', () => response));

    await page.goto('/crates/foo');
    await expect(page.locator('[data-test-source-link] a')).toHaveAttribute(
      'href',
      'https://docs.rs/crate/foo/0.6.2/source/',
    );
  });
});
