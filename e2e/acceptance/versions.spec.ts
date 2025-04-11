import { expect, test } from '@/e2e/helper';

test.describe('Acceptance | crate versions page', { tag: '@acceptance' }, () => {
  test('show versions sorted by date', async ({ page, msw, percy }) => {
    let crate = msw.db.crate.create({ name: 'nanomsg' });
    msw.db.version.create({ crate, num: '0.1.0', created_at: '2017-01-01' });
    msw.db.version.create({ crate, num: '0.2.0', created_at: '2018-01-01' });
    msw.db.version.create({ crate, num: '0.3.0', created_at: '2019-01-01', rust_version: '1.69' });
    msw.db.version.create({ crate, num: '0.2.1', created_at: '2020-01-01' });

    await page.goto('/crates/nanomsg/versions');
    await expect(page).toHaveURL('/crates/nanomsg/versions');

    await expect(page.locator('[data-test-version]')).toHaveCount(4);
    let versions = await page.locator('[data-test-version]').evaluateAll(el => el.map(it => it.dataset.testVersion));
    expect(versions).toEqual(['0.2.1', '0.3.0', '0.2.0', '0.1.0']);

    await percy.snapshot();

    await page.click('[data-test-current-order]');
    await page.click('[data-test-semver-sort] a');

    await expect(page.locator('[data-test-version]').first()).toBeVisible();
    versions = await page.locator('[data-test-version]').evaluateAll(el => el.map(it => it.dataset.testVersion));
    expect(versions).toEqual(['0.3.0', '0.2.1', '0.2.0', '0.1.0']);
  });

  test('shows correct release tracks label after yanking/unyanking', async ({ page, msw, percy }) => {
    let user = msw.db.user.create();
    await msw.authenticateAs(user);

    let crate = msw.db.crate.create({ name: 'nanomsg' });
    msw.db.crateOwnership.create({ crate, user });

    msw.db.version.create({ crate, num: '0.1.0', created_at: '2017-01-01' });
    msw.db.version.create({ crate, num: '0.2.0', created_at: '2018-01-01' });
    msw.db.version.create({ crate, num: '0.3.0', created_at: '2019-01-01', rust_version: '1.69' });
    msw.db.version.create({ crate, num: '0.2.1', created_at: '2020-01-01' });

    await page.goto('/crates/nanomsg/versions');
    await expect(page).toHaveURL('/crates/nanomsg/versions');

    await expect(page.locator('[data-test-version]')).toHaveCount(4);
    let versions = await page.locator('[data-test-version]').evaluateAll(el => el.map(it => it.dataset.testVersion));
    expect(versions).toEqual(['0.2.1', '0.3.0', '0.2.0', '0.1.0']);

    let v021 = page.locator('[data-test-version="0.2.1"]');
    let v020 = page.locator('[data-test-version="0.2.0"]');

    await expect(v021).toHaveClass(/.*latest/);
    await expect(v021).not.toHaveClass(/.yanked/);
    await expect(v020).not.toHaveClass(/.*latest/);
    await expect(v020).not.toHaveClass(/.yanked/);

    // yanking
    await page.locator('[data-test-version-yank-button="0.2.1"]').click();
    await expect(v021).not.toHaveClass(/.*latest/);
    await expect(v021).toHaveClass(/.yanked/);
    await expect(v020).toHaveClass(/.*latest/);
    await expect(v020).not.toHaveClass(/.yanked/);

    // unyanking
    await page.locator('[data-test-version-unyank-button="0.2.1"]').click();
    await expect(v021).toHaveClass(/.*latest/);
    await expect(v021).not.toHaveClass(/.yanked/);
    await expect(v020).not.toHaveClass(/.*latest/);
    await expect(v020).not.toHaveClass(/.yanked/);
  });
});
