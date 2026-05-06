import type { AppFixtures } from '@/e2e/helper';

import { expect, test } from '@/e2e/helper';

test.describe('Route | user | yanked crates visibility', { tag: '@routes' }, () => {
  async function prepare(msw: AppFixtures['msw']) {
    let user1 = await msw.db.user.create({ login: 'alice', name: 'Alice' });
    let user2 = await msw.db.user.create({ login: 'bob', name: 'Bob' });

    // Alice owns "alpha" (not yanked) and "alpha-yanked" (all versions yanked)
    let alpha = await msw.db.crate.create({ name: 'alpha' });
    await msw.db.version.create({ crate: alpha, num: '1.0.0' });
    await msw.db.crateOwnership.create({ crate: alpha, user: user1 });

    let alphaYanked = await msw.db.crate.create({ name: 'alpha-yanked' });
    await msw.db.version.create({ crate: alphaYanked, num: '1.0.0', yanked: true });
    await msw.db.crateOwnership.create({ crate: alphaYanked, user: user1 });

    // Bob owns "bravo" (not yanked) and "bravo-yanked" (all versions yanked)
    let bravo = await msw.db.crate.create({ name: 'bravo' });
    await msw.db.version.create({ crate: bravo, num: '1.0.0' });
    await msw.db.crateOwnership.create({ crate: bravo, user: user2 });

    let bravoYanked = await msw.db.crate.create({ name: 'bravo-yanked' });
    await msw.db.version.create({ crate: bravoYanked, num: '1.0.0', yanked: true });
    await msw.db.crateOwnership.create({ crate: bravoYanked, user: user2 });

    return { user1, user2 };
  }

  test('own profile shows yanked crates', async ({ page, msw }) => {
    let { user1 } = await prepare(msw);
    await msw.authenticateAs(user1);

    await page.goto('/users/alice');
    await expect(page.locator('[data-test-crate-row]')).toHaveCount(2);
    await expect(page.locator('[data-test-crate-link]')).toContainText(['alpha', 'alpha-yanked']);
  });

  test("other user's profile hides yanked crates", async ({ page, msw }) => {
    let { user1 } = await prepare(msw);
    await msw.authenticateAs(user1);

    await page.goto('/users/bob');
    await expect(page.locator('[data-test-crate-row]')).toHaveCount(1);
    await expect(page.locator('[data-test-crate-link]')).toContainText(['bravo']);
  });

  test('unauthenticated view hides yanked crates', async ({ page, msw }) => {
    await prepare(msw);

    await page.goto('/users/alice');
    await expect(page.locator('[data-test-crate-row]')).toHaveCount(1);
    await expect(page.locator('[data-test-crate-link]')).toContainText(['alpha']);
  });
});
