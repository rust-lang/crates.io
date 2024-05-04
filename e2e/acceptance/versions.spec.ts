import { test, expect } from '@/e2e/helper';

test.describe('Acceptance | crate versions page', { tag: '@acceptance' }, () => {
  test('show versions sorted by date', async ({ page, mirage, percy }) => {
    await mirage.addHook(server => {
      let crate = server.create('crate', { name: 'nanomsg' });
      server.create('version', { crate, num: '0.1.0', created_at: '2017-01-01' });
      server.create('version', { crate, num: '0.2.0', created_at: '2018-01-01' });
      server.create('version', { crate, num: '0.3.0', created_at: '2019-01-01', rust_version: '1.69' });
      server.create('version', { crate, num: '0.2.1', created_at: '2020-01-01' });
    });

    await page.goto('/crates/nanomsg/versions');
    await expect(page).toHaveURL('/crates/nanomsg/versions');

    let versions = await page.locator('[data-test-version]').evaluateAll(el => el.map(it => it.dataset.testVersion));
    expect(versions).toEqual(['0.2.1', '0.3.0', '0.2.0', '0.1.0']);

    await percy.snapshot();

    await page.click('[data-test-current-order]');
    await page.click('[data-test-semver-sort] a');

    await expect(page.locator('[data-test-version]').first()).toBeVisible();
    versions = await page.locator('[data-test-version]').evaluateAll(el => el.map(it => it.dataset.testVersion));
    expect(versions).toEqual(['0.3.0', '0.2.1', '0.2.0', '0.1.0']);
  });
});
