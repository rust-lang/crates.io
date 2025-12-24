import { expect, test } from '@/e2e/helper';
import { http, HttpResponse } from 'msw';

test.describe('Acceptance | /crates/:crate_id/reverse_dependencies', { tag: '@acceptance' }, () => {
  async function prepare(msw) {
    let foo = await msw.db.crate.create({ name: 'foo' });
    await msw.db.version.create({ crate: foo });

    let bar = await msw.db.crate.create({ name: 'bar' });
    let barV = await msw.db.version.create({ crate: bar });

    let baz = await msw.db.crate.create({ name: 'baz' });
    let bazV = await msw.db.version.create({ crate: baz });

    await msw.db.dependency.create({ crate: foo, version: barV });
    await msw.db.dependency.create({ crate: foo, version: bazV });

    return { foo, bar, baz };
  }

  test('shows a list of crates depending on the selected crate', async ({ page, msw }) => {
    let { foo, bar, baz } = await prepare(msw);

    await page.goto(`/crates/${foo.name}/reverse_dependencies`);
    await expect(page).toHaveURL(`/crates/${foo.name}/reverse_dependencies`);

    await expect(page.locator('[data-test-row]')).toHaveCount(2);
    const row0 = page.locator('[data-test-row="0"]');
    await expect(row0.locator('[data-test-crate-name]')).toHaveText(baz.name);
    await expect(row0.locator('[data-test-description]')).toHaveText(baz.description);
    const row1 = page.locator('[data-test-row="1"]');
    await expect(row1.locator('[data-test-crate-name]')).toHaveText(bar.name);
    await expect(row1.locator('[data-test-description]')).toHaveText(bar.description);
  });

  test('supports pagination', async ({ page, msw }) => {
    let { foo } = await prepare(msw);

    for (let i = 0; i < 20; i++) {
      let crate = await msw.db.crate.create({});
      let version = await msw.db.version.create({ crate });
      await msw.db.dependency.create({ crate: foo, version });
    }

    const row = page.locator('[data-test-row]');
    const currentRows = page.locator('[data-test-current-rows]');
    const totalRows = page.locator('[data-test-total-rows]');

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

  test('shows error message if loading of reverse dependencies fails', async ({ page, msw }) => {
    let { foo } = await prepare(msw);

    let error = HttpResponse.json({}, { status: 500 });
    await msw.worker.use(http.get('/api/v1/crates/:crate_id/reverse_dependencies', () => error));

    await page.goto(`/crates/${foo.name}/reverse_dependencies`);
    await expect(page).toHaveURL(`/crates/${foo.name}/reverse_dependencies`);
    await expect(page.locator('[data-test-404-page]')).toBeVisible();
    await expect(page.locator('[data-test-title]')).toHaveText(`${foo.name}: Failed to load dependents`);
    await expect(page.locator('[data-test-go-back]')).toBeVisible();
    await expect(page.locator('[data-test-try-again]')).not.toBeVisible();
  });
});
