import { expect, test } from '@/e2e/helper';
import { http, HttpResponse } from 'msw';

test.describe('Acceptance | crate security page', { tag: '@acceptance' }, () => {
  test('show some advisories', async ({ page, msw, percy }) => {
    let crate = await msw.db.crate.create({ name: 'foo' });
    await msw.db.version.create({ crate, num: '1.0.0' });

    let advisories = [
      {
        id: 'TEST-001',
        summary: 'First test advisory',
        details: 'This is the first test advisory with **markdown** support.',
      },
      {
        id: 'TEST-002',
        summary: 'Second test advisory',
        details: 'This is the second test advisory with more details.',
      },
    ];

    msw.worker.use(http.get('https://rustsec.org/packages/:crateId.json', () => HttpResponse.json(advisories)));

    await page.goto('/crates/foo/security');

    await expect(page.locator('[data-test-list] li')).toHaveCount(2);

    // Check first advisory
    await expect(page.locator('[data-test-list] li').nth(0).locator('h3 a')).toHaveAttribute(
      'href',
      'https://rustsec.org/advisories/TEST-001.html',
    );
    await expect(page.locator('[data-test-list] li').nth(0).locator('h3 a')).toContainText('TEST-001');
    await expect(page.locator('[data-test-list] li').nth(0).locator('h3')).toContainText('First test advisory');
    await expect(page.locator('[data-test-list] li').nth(0).locator('p')).toContainText('markdown');

    // Check second advisory
    await expect(page.locator('[data-test-list] li').nth(1).locator('h3 a')).toHaveAttribute(
      'href',
      'https://rustsec.org/advisories/TEST-002.html',
    );
    await expect(page.locator('[data-test-list] li').nth(1).locator('h3 a')).toContainText('TEST-002');
    await expect(page.locator('[data-test-list] li').nth(1).locator('h3')).toContainText('Second test advisory');

    await percy.snapshot();
  });

  test('show no advisory data when none exist', async ({ page, msw }) => {
    let crate = await msw.db.crate.create({ name: 'safe-crate' });
    await msw.db.version.create({ crate, num: '1.0.0' });

    msw.worker.use(
      http.get('https://rustsec.org/packages/:crateId.json', () => HttpResponse.text('not found', { status: 404 })),
    );

    await page.goto('/crates/safe-crate/security');

    await expect(page.locator('[data-no-advisories]')).toBeVisible();
    await expect(page.locator('[data-no-advisories]')).toHaveText('No advisories found for this crate.');
  });

  test('handles errors gracefully', async ({ page, msw }) => {
    let crate = await msw.db.crate.create({ name: 'error-crate' });
    await msw.db.version.create({ crate, num: '1.0.0' });

    msw.worker.use(
      http.get('https://rustsec.org/packages/:crateId.json', () =>
        HttpResponse.text('Internal Server Error', { status: 500 }),
      ),
    );

    await page.goto('/crates/error-crate/security');

    // When there's an error, the route catches it and returns empty advisories
    await expect(page.locator('[data-error]')).toBeVisible();
    await expect(page.locator('[data-error]')).toHaveText('An error occurred while fetching advisories.');
  });
});
