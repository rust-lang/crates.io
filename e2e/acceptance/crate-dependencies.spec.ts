import { test, expect } from '@/e2e/helper';

test.describe('Acceptance | crate dependencies page', { tag: '@acceptance' }, () => {
  test('shows the lists of dependencies', async ({ page, mirage, percy, a11y }) => {
    await mirage.addHook(server => {
      server.loadFixtures();
    });

    await page.goto('/crates/nanomsg/dependencies');
    await expect(page).toHaveURL('/crates/nanomsg/0.6.1/dependencies');
    expect(await page.title()).toBe('nanomsg - crates.io: Rust Package Registry');

    await expect(page.locator('[data-test-dependencies] li')).toHaveCount(2);
    await expect(page.locator('[data-test-build-dependencies] li')).toHaveCount(1);
    await expect(page.locator('[data-test-dev-dependencies] li')).toHaveCount(1);

    await percy.snapshot();
    await a11y.audit();
  });

  test('empty list case', async ({ page, mirage }) => {
    await mirage.addHook(server => {
      let crate = server.create('crate', { name: 'nanomsg' });
      server.create('version', { crate, num: '0.6.1' });
    });

    await page.goto('/crates/nanomsg/dependencies');

    await expect(page.locator('[data-test-no-dependencies]')).toBeVisible();
    await expect(page.locator('[data-test-dependencies] li')).toHaveCount(0);
    await expect(page.locator('[data-test-build-dependencies] li')).toHaveCount(0);
    await expect(page.locator('[data-test-dev-dependencies] li')).toHaveCount(0);
  });

  test('shows an error page if crate not found', async ({ page }) => {
    await page.goto('/crates/foo/1.0.0/dependencies');
    await expect(page).toHaveURL('/crates/foo/1.0.0/dependencies');
    await expect(page.locator('[data-test-404-page]')).toBeVisible();
    await expect(page.locator('[data-test-title]')).toHaveText('foo: Crate not found');
    await expect(page.locator('[data-test-go-back]')).toBeVisible();
    await expect(page.locator('[data-test-try-again]')).toHaveCount(0);
  });

  test('shows an error page if crate fails to load', async ({ page, mirage }) => {
    await mirage.addHook(server => {
      server.get('/api/v1/crates/:crate_name', {}, 500);
    });

    await page.goto('/crates/foo/1.0.0/dependencies');
    await expect(page).toHaveURL('/crates/foo/1.0.0/dependencies');
    await expect(page.locator('[data-test-404-page]')).toBeVisible();
    await expect(page.locator('[data-test-title]')).toHaveText('foo: Failed to load crate data');
    await expect(page.locator('[data-test-go-back]')).toHaveCount(0);
    await expect(page.locator('[data-test-try-again]')).toBeVisible();
  });

  test('shows an error page if version is not found', async ({ page, mirage }) => {
    await mirage.addHook(server => {
      let crate = server.create('crate', { name: 'foo' });
      server.create('version', { crate, num: '2.0.0' });
    });

    await page.goto('/crates/foo/1.0.0/dependencies');
    await expect(page).toHaveURL('/crates/foo/1.0.0/dependencies');
    await expect(page.locator('[data-test-404-page]')).toBeVisible();
    await expect(page.locator('[data-test-title]')).toHaveText('foo: Version 1.0.0 not found');
    await expect(page.locator('[data-test-go-back]')).toBeVisible();
    await expect(page.locator('[data-test-try-again]')).toHaveCount(0);
  });

  test('shows an error page if versions fail to load', async ({ page, mirage, ember }) => {
    await mirage.addHook(server => {
      let crate = server.create('crate', { name: 'foo' });
      server.create('version', { crate, num: '2.0.0' });
      server.get('/api/v1/crates/:crate_name/versions', {}, 500);
    });

    await ember.addHook(async owner => {
      // Load `crate` and then explicitly unload the side-loaded `versions`.
      let store = owner.lookup('service:store');
      let crateRecord = await store.findRecord('crate', 'foo');
      let versions = crateRecord.hasMany('versions').value();
      versions.forEach(record => record.unloadRecord());
    });

    await page.goto('/crates/foo/1.0.0/dependencies');

    await expect(page).toHaveURL('/crates/foo/1.0.0/dependencies');
    await expect(page.locator('[data-test-404-page]')).toBeVisible();
    await expect(page.locator('[data-test-title]')).toHaveText('foo: Failed to load version data');
    await expect(page.locator('[data-test-go-back]')).toHaveCount(0);
    await expect(page.locator('[data-test-try-again]')).toBeVisible();
  });

  test('shows error message if loading of dependencies fails', async ({ page, mirage }) => {
    await mirage.addHook(server => {
      let crate = server.create('crate', { name: 'foo' });
      server.create('version', { crate, num: '1.0.0' });

      server.get('/api/v1/crates/:crate_name/:version_num/dependencies', {}, 500);
    });

    await page.goto('/crates/foo/1.0.0/dependencies');
    await expect(page).toHaveURL('/crates/foo/1.0.0/dependencies');
    await expect(page.locator('[data-test-404-page]')).toBeVisible();
    await expect(page.locator('[data-test-title]')).toHaveText('foo: Failed to load dependencies');
    await expect(page.locator('[data-test-go-back]')).toHaveCount(0);
    await expect(page.locator('[data-test-try-again]')).toBeVisible();
  });

  test('hides description if loading of dependency details fails', async ({ page, mirage }) => {
    await mirage.addHook(server => {
      let crate = server.create('crate', { name: 'nanomsg' });
      let version = server.create('version', { crate, num: '0.6.1' });

      let foo = server.create('crate', { name: 'foo', description: 'This is the foo crate' });
      server.create('version', { crate: foo, num: '1.0.0' });
      server.create('dependency', { crate: foo, version, req: '^1.0.0', kind: 'normal' });

      let bar = server.create('crate', { name: 'bar', description: 'This is the bar crate' });
      server.create('version', { crate: bar, num: '2.3.4' });
      server.create('dependency', { crate: bar, version, req: '^2.0.0', kind: 'normal' });

      server.get('/api/v1/crates', {}, 500);
    });

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
