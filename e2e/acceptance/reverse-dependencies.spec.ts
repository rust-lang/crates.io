import { test, expect } from '@/e2e/helper';

test.describe('Acceptance | /crates/:crate_id/reverse_dependencies', { tag: '@acceptance' }, () => {
  test.beforeEach(async ({ page, mirage }) => {
    await page.addInitScript(() => {
      globalThis.foo = { name: 'foo' };
      globalThis.bar = { name: 'bar' };
      globalThis.baz = { name: 'baz' };
    });
    await mirage.addHook(server => {
      console.log('[>>>] mirage');
      let foo = server.create('crate', globalThis.foo);
      server.create('version', { crate: foo });

      let bar = server.create('crate', globalThis.bar);
      server.create('version', { crate: bar });

      let baz = server.create('crate', globalThis.baz);
      server.create('version', { crate: baz });

      server.create('dependency', { crate: foo, version: bar.versions.models[0] });
      server.create('dependency', { crate: foo, version: baz.versions.models[0] });

      globalThis.foo = foo;
      globalThis.bar = bar;
      globalThis.baz = baz;
    });

    // this allows us to evaluate the name before goingo to the actual page
    await page.goto('about:blank');
  });

  test('shows a list of crates depending on the selected crate', async ({ page }) => {
    const foo = await page.evaluate(() => globalThis.foo);

    await page.goto(`/crates/${foo.name}/reverse_dependencies`);
    await expect(page).toHaveURL(`/crates/${foo.name}/reverse_dependencies`);

    const { bar, baz } = await page.evaluate(() => {
      const val = item => ({ name: item.name, description: item.description });
      return { bar: val(bar), baz: val(baz) };
    });

    await expect(page.locator('[data-test-row]')).toHaveCount(2);
    const row0 = page.locator('[data-test-row="0"]');
    await expect(row0.locator('[data-test-crate-name]')).toHaveText(bar.name);
    await expect(row0.locator('[data-test-description]')).toHaveText(bar.description);
    const row1 = page.locator('[data-test-row="1"]');
    await expect(row1.locator('[data-test-crate-name]')).toHaveText(baz.name);
    await expect(row1.locator('[data-test-description]')).toHaveText(baz.description);
  });

  test('supports pagination', async ({ page, mirage }) => {
    await mirage.addHook(server => {
      let foo = globalThis.foo;

      for (let i = 0; i < 20; i++) {
        let crate = server.create('crate');
        let version = server.create('version', { crate });
        server.create('dependency', { crate: foo, version });
      }
    });

    const row = page.locator('[data-test-row]');
    const currentRows = page.locator('[data-test-current-rows]');
    const totalRows = page.locator('[data-test-total-rows]');

    const foo = await page.evaluate(() => globalThis.foo);
    await page.goto(`/crates/${foo.name}/reverse_dependencies`);
    await expect(page).toHaveURL(`/crates/${foo.name}/reverse_dependencies`);
    await expect(row).toHaveCount(10);
    await expect(currentRows).toHaveText('1-10');
    await expect(totalRows).toHaveText('22');

    await page.click('[data-test-pagination-next]');
    await expect(page).toHaveURL(`/crates/${foo.name}/reverse_dependencies?page=2`);
    await expect(row).toHaveCount(10);
    await expect(currentRows).toHaveText('11-20');
    await expect(totalRows).toHaveText('22');

    await page.click('[data-test-pagination-next]');
    await expect(page).toHaveURL(`/crates/${foo.name}/reverse_dependencies?page=3`);
    await expect(row).toHaveCount(2);
    await expect(currentRows).toHaveText('21-22');
    await expect(totalRows).toHaveText('22');
  });

  test('shows a generic error if the server is broken', async ({ page, mirage }) => {
    await mirage.addHook(server => {
      server.get('/api/v1/crates/:crate_id/reverse_dependencies', {}, 500);
    });

    const foo = await page.evaluate(() => globalThis.foo);

    await page.goto(`/crates/${foo.name}/reverse_dependencies`);
    await expect(page).toHaveURL('/');
    await expect(page.locator('[data-test-notification-message="error"]')).toHaveText(
      'Could not load reverse dependencies for the "foo" crate',
    );
  });

  test('shows a detailed error if available', async ({ page, mirage }) => {
    await mirage.addHook(server => {
      let payload = { errors: [{ detail: 'cannot request more than 100 items' }] };
      server.get('/api/v1/crates/:crate_id/reverse_dependencies', payload, 400);
    });

    const foo = await page.evaluate(() => globalThis.foo);

    await page.goto(`/crates/${foo.name}/reverse_dependencies`);
    await expect(page).toHaveURL('/');
    await expect(page.locator('[data-test-notification-message="error"]')).toHaveText(
      'Could not load reverse dependencies for the "foo" crate: cannot request more than 100 items',
    );
  });
});
