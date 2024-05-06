import { test, expect } from '@/e2e/helper';

test.describe('Route | crate.version | docs link', { tag: '@routes' }, () => {
  test('shows regular documentation link', async ({ page, mirage }) => {
    await mirage.addHook(server => {
      let crate = server.create('crate', { name: 'foo', documentation: 'https://foo.io/docs' });
      server.create('version', { crate, num: '1.0.0' });
    });

    await page.goto('/crates/foo');
    await expect(page.locator('[data-test-docs-link] a')).toHaveAttribute('href', 'https://foo.io/docs');
  });

  test('show no docs link if `documentation` is unspecified and there are no related docs.rs builds', async ({
    page,
    mirage,
  }) => {
    await mirage.addHook(server => {
      let crate = server.create('crate', { name: 'foo' });
      server.create('version', { crate, num: '1.0.0' });

      server.get('https://docs.rs/crate/:crate/:version/status.json', 'not found', 404);
    });

    await page.goto('/crates/foo');
    await expect(page.getByRole('link', { name: 'crates.io', exact: true })).toHaveCount(1);

    await expect(page.locator('[data-test-docs-link] a')).toHaveCount(0);
  });

  test('show docs link if `documentation` is unspecified and there are related docs.rs builds', async ({
    page,
    mirage,
  }) => {
    await mirage.addHook(server => {
      let crate = server.create('crate', { name: 'foo' });
      server.create('version', { crate, num: '1.0.0' });

      server.get('https://docs.rs/crate/:crate/:version/status.json', {
        doc_status: true,
        version: '1.0.0',
      });
    });

    await page.goto('/crates/foo');
    await expect(page.locator('[data-test-docs-link] a')).toHaveAttribute('href', 'https://docs.rs/foo/1.0.0');
  });

  test('show original docs link if `documentation` points to docs.rs and there are no related docs.rs builds', async ({
    page,
    mirage,
  }) => {
    await mirage.addHook(server => {
      let crate = server.create('crate', { name: 'foo', documentation: 'https://docs.rs/foo/0.6.2' });
      server.create('version', { crate, num: '1.0.0' });

      server.get('https://docs.rs/crate/:crate/:version/status.json', 'not found', 404);
    });

    await page.goto('/crates/foo');
    await expect(page.locator('[data-test-docs-link] a')).toHaveAttribute('href', 'https://docs.rs/foo/0.6.2');
  });

  test('show updated docs link if `documentation` points to docs.rs and there are related docs.rs builds', async ({
    page,
    mirage,
  }) => {
    await mirage.addHook(server => {
      let crate = server.create('crate', { name: 'foo', documentation: 'https://docs.rs/foo/0.6.2' });
      server.create('version', { crate, num: '1.0.0' });

      server.get('https://docs.rs/crate/:crate/:version/status.json', {
        doc_status: true,
        version: '1.0.0',
      });
    });

    await page.goto('/crates/foo');
    await expect(page.locator('[data-test-docs-link] a')).toHaveAttribute('href', 'https://docs.rs/foo/1.0.0');
  });

  test('ajax errors are ignored', async ({ page, mirage }) => {
    await mirage.addHook(server => {
      let crate = server.create('crate', { name: 'foo', documentation: 'https://docs.rs/foo/0.6.2' });
      server.create('version', { crate, num: '1.0.0' });

      server.get('https://docs.rs/crate/:crate/:version/status.json', 'error', 500);
    });

    await page.goto('/crates/foo');
    await expect(page.locator('[data-test-docs-link] a')).toHaveAttribute('href', 'https://docs.rs/foo/0.6.2');
  });

  test('empty docs.rs responses are ignored', async ({ page, mirage }) => {
    await mirage.addHook(server => {
      let crate = server.create('crate', { name: 'foo', documentation: 'https://docs.rs/foo/0.6.2' });
      server.create('version', { crate, num: '0.6.2' });

      server.get('https://docs.rs/crate/:crate/:version/status.json', {});
    });

    await page.goto('/crates/foo');
    await expect(page.locator('[data-test-docs-link] a')).toHaveAttribute('href', 'https://docs.rs/foo/0.6.2');
  });
});
