import { expect, test } from '@/e2e/helper';
import { loadFixtures } from '@crates-io/msw/fixtures';
import { http, HttpResponse } from 'msw';

test.describe('Acceptance | crate dependencies page', { tag: '@acceptance' }, () => {
  test('shows the lists of dependencies', async ({ page, msw, percy, a11y }) => {
    await loadFixtures(msw.db);

    await page.goto('/crates/nanomsg/dependencies');
    await expect(page).toHaveURL('/crates/nanomsg/0.6.1/dependencies');
    expect(await page.title()).toBe('nanomsg - crates.io: Rust Package Registry');

    await expect(page.locator('[data-test-dependencies] li')).toHaveCount(2);
    await expect(page.locator('[data-test-build-dependencies] li')).toHaveCount(1);
    await expect(page.locator('[data-test-dev-dependencies] li')).toHaveCount(1);

    await percy.snapshot();
    await a11y.audit();
  });

  test('empty list case', async ({ page, msw }) => {
    let crate = await msw.db.crate.create({ name: 'nanomsg' });
    await msw.db.version.create({ crate, num: '0.6.1' });

    await page.goto('/crates/nanomsg/dependencies');

    await expect(page.locator('[data-test-no-dependencies]')).toBeVisible();
    await expect(page.locator('[data-test-dependencies] li')).toHaveCount(0);
    await expect(page.locator('[data-test-build-dependencies] li')).toHaveCount(0);
    await expect(page.locator('[data-test-dev-dependencies] li')).toHaveCount(0);
  });

  test('shows an error page if crate not found', async ({ page, msw }) => {
    void msw;

    await page.goto('/crates/foo/1.0.0/dependencies');
    await expect(page).toHaveURL('/crates/foo/1.0.0/dependencies');
    await expect(page.locator('[data-test-404-page]')).toBeVisible();
    await expect(page.locator('[data-test-title]')).toHaveText(`Crate "foo" not found`);
    await expect(page.locator('[data-test-go-back]')).toBeVisible();
    await expect(page.locator('[data-test-try-again]')).toHaveCount(0);
  });

  test('shows an error page if crate fails to load', async ({ page, msw }) => {
    await msw.worker.use(http.get('/api/v1/crates/:crate_name', () => HttpResponse.json({}, { status: 500 })));

    await page.goto('/crates/foo/1.0.0/dependencies');
    await expect(page).toHaveURL('/crates/foo/1.0.0/dependencies');
    await expect(page.locator('[data-test-404-page]')).toBeVisible();
    await expect(page.locator('[data-test-title]')).toHaveText(`Failed to load crate data`);
    await expect(page.locator('[data-test-go-back]')).toHaveCount(0);
    await expect(page.locator('[data-test-try-again]')).toBeVisible();
  });

  test('shows an error page if version is not found', async ({ page, msw }) => {
    let crate = await msw.db.crate.create({ name: 'foo' });
    await msw.db.version.create({ crate, num: '2.0.0' });

    await page.goto('/crates/foo/1.0.0/dependencies');
    await expect(page).toHaveURL('/crates/foo/1.0.0/dependencies');
    await expect(page.locator('[data-test-404-page]')).toBeVisible();
    await expect(page.locator('[data-test-title]')).toHaveText('foo: Version 1.0.0 not found');
    await expect(page.locator('[data-test-go-back]')).toBeVisible();
    await expect(page.locator('[data-test-try-again]')).toHaveCount(0);
  });

  test('shows error message if loading of dependencies fails', async ({ page, msw }) => {
    let crate = await msw.db.crate.create({ name: 'foo' });
    await msw.db.version.create({ crate, num: '1.0.0' });

    let error = HttpResponse.json({}, { status: 500 });
    await msw.worker.use(http.get('/api/v1/crates/:crate_name/:version_num/dependencies', () => error));

    await page.goto('/crates/foo/1.0.0/dependencies');
    await expect(page).toHaveURL('/crates/foo/1.0.0/dependencies');
    await expect(page.locator('[data-test-404-page]')).toBeVisible();
    await expect(page.locator('[data-test-title]')).toHaveText('foo: Failed to load dependencies');
    await expect(page.locator('[data-test-go-back]')).toHaveCount(0);
    await expect(page.locator('[data-test-try-again]')).toBeVisible();
  });

  test('hides description if loading of dependency details fails', async ({ page, msw }) => {
    let crate = await msw.db.crate.create({ name: 'nanomsg' });
    let version = await msw.db.version.create({ crate, num: '0.6.1' });

    let foo = await msw.db.crate.create({ name: 'foo', description: 'This is the foo crate' });
    await msw.db.version.create({ crate: foo, num: '1.0.0' });
    await msw.db.dependency.create({ crate: foo, version, req: '^1.0.0', kind: 'normal' });

    let bar = await msw.db.crate.create({ name: 'bar', description: 'This is the bar crate' });
    await msw.db.version.create({ crate: bar, num: '2.3.4' });
    await msw.db.dependency.create({ crate: bar, version, req: '^2.0.0', kind: 'normal' });

    let error = HttpResponse.json({}, { status: 500 });
    await msw.worker.use(http.get('/api/v1/crates', () => error));

    await page.goto('/crates/nanomsg/dependencies');
    await expect(page).toHaveURL('/crates/nanomsg/0.6.1/dependencies');

    await expect(page.locator('[data-test-dependencies] li')).toHaveCount(2);

    await expect(page.locator('[data-test-dependency="foo"]')).toBeVisible();
    await expect(page.locator('[data-test-dependency="foo"] [data-test-crate-name]')).toHaveText('foo');
    await expect(page.locator('[data-test-dependency="bar"] [data-test-description]')).toHaveCount(0);

    await expect(page.locator('[data-test-dependency="bar"]')).toBeVisible();
    await expect(page.locator('[data-test-dependency="bar"] [data-test-crate-name]')).toHaveText('bar');
    await expect(page.locator('[data-test-dependency="bar"] [data-test-description]')).toHaveCount(0);
  });
});
