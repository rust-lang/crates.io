import { test, expect } from '@/e2e/helper';

test.describe('Bug #11772', { tag: '@bugs' }, () => {
  async function prepare(msw: any) {
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
    await expect(page.locator('[data-test-new-crates] [data-test-crate-link]')).toContainText('test-crate v1.2.3');
    await expect(page.locator('[data-test-just-updated] [data-test-crate-link]')).toContainText('test-crate v1.2.3');

    // Click on a crate to navigate to its details page
    await page.click('[data-test-new-crates] [data-test-crate-link]');

    // Verify we're on the crate details page
    await expect(page).toHaveURL('/crates/test-crate');

    await page.goto('/'); // Re-visit to simulate the back navigation

    // Versions should still be displayed correctly, not v0.0.0
    await expect(page.locator('[data-test-new-crates] [data-test-crate-link]')).toContainText('test-crate v1.2.3');
    await expect(page.locator('[data-test-just-updated] [data-test-crate-link]')).toContainText('test-crate v1.2.3');
  });

  test('crates with actual v0.0.0 versions should display correctly', async ({ page, msw }) => {
    // Create a crate with an actual v0.0.0 version
    let zeroCrate = await msw.db.crate.create({ name: 'test-zero-crate' });
    await msw.db.version.create({ crate: zeroCrate, num: '0.0.0' });

    // Visit homepage
    await page.goto('/');
    await expect(page).toHaveURL('/');

    // Should correctly display v0.0.0 for crates that actually have that version
    await expect(page.locator('[data-test-new-crates] [data-test-crate-link]')).toContainText('test-zero-crate v0.0.0');

    // Click on the crate to navigate to its details page
    await page.click('[data-test-new-crates] [data-test-crate-link]');

    // Verify we're on the crate details page
    await expect(page).toHaveURL('/crates/test-zero-crate');

    await page.goto('/'); // Re-visit to simulate the back navigation

    // Should still display v0.0.0 correctly (this is the intended behavior)
    await expect(page.locator('[data-test-new-crates] [data-test-crate-link]')).toContainText('test-zero-crate v0.0.0');
  });
});
