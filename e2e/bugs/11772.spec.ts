import type { AppFixtures } from '@/e2e/helper';

import { expect, test } from '@/e2e/helper';

test.describe('Bug #11772', { tag: '@bugs' }, () => {
  async function prepare(msw: AppFixtures['msw']) {
    // Create a crate that will appear in "New Crates" section
    let newCrate = await msw.db.crate.create({ name: 'test-crate' });
    await msw.db.version.create({ crate: newCrate, num: '1.2.3' });
  }

  test('crate versions should remain correct after navigating back from crate details', async ({ page, msw }) => {
    await prepare(msw);

    // Visit homepage
    await page.goto('/');
    await expect(page).toHaveURL('/');

    // Verify initial correct version displays
    await expectVersion(page, '1.2.3');

    // Click on a crate to navigate to its details page
    await page.click('[data-test-new-crates] [data-test-crate-link]');

    // Verify we're on the crate details page
    await expect(page).toHaveURL('/crates/test-crate');

    await page.goto('/'); // Re-visit to simulate the back navigation

    // Versions should still be displayed correctly, not v0.0.0
    await expectVersion(page, '1.2.3');
  });

  test('crates with actual v0.0.0 versions should display correctly', async ({ page, msw }) => {
    // Create a crate with an actual v0.0.0 version
    let zeroCrate = await msw.db.crate.create({ name: 'test-zero-crate' });
    await msw.db.version.create({ crate: zeroCrate, num: '0.0.0' });

    // Visit homepage
    await page.goto('/');
    await expect(page).toHaveURL('/');

    // Should correctly display 0.0.0 for crates that actually have that version
    await expectVersion(page, '0.0.0', 'test-zero-crate');

    // Click on the crate to navigate to its details page
    await page.click('[data-test-new-crates] [data-test-crate-link]');

    // Verify we're on the crate details page
    await expect(page).toHaveURL('/crates/test-zero-crate');

    await page.goto('/'); // Re-visit to simulate the back navigation

    // Should still display 0.0.0 correctly (this is the intended behavior)
    await expectVersion(page, '0.0.0', 'test-zero-crate');
  });
});

async function expectVersion(page: AppFixtures['page'], version: string, name = 'test-crate') {
  for (let section of ['new-crates', 'just-updated']) {
    let link = page.locator(`[data-test-${section}] [data-test-crate-link]`);
    await expect(link.locator('[data-test-title]')).toHaveText(name);
    await expect(link.locator('[data-test-version]')).toHaveText(version);
  }
}
