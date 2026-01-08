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

    await msw.worker.use(http.get('https://rustsec.org/packages/:crateId.json', () => HttpResponse.json(advisories)));

    await page.goto('/crates/foo/security');

    await expect(page.locator('[data-test-list] li')).toHaveCount(2);

    // Check first advisory
    let advisory1 = page.locator('[data-test-list] li').nth(0);
    await expect(advisory1.locator('h3 a')).toHaveAttribute('href', 'https://rustsec.org/advisories/TEST-001.html');
    await expect(advisory1.locator('h3 a')).toHaveText('TEST-001');
    await expect(advisory1.locator('h3')).toContainText('First test advisory');
    expect(await advisory1.locator('p').innerHTML()).toBe(
      'This is the first test advisory with <strong>markdown</strong> support.',
    );

    // Check second advisory
    let advisory2 = page.locator('[data-test-list] li').nth(1);
    await expect(advisory2.locator('h3 a')).toHaveAttribute('href', 'https://rustsec.org/advisories/TEST-002.html');
    await expect(advisory2.locator('h3 a')).toHaveText('TEST-002');
    await expect(advisory2.locator('h3')).toContainText('Second test advisory');
    expect(await advisory2.locator('p').innerHTML()).toBe('This is the second test advisory with more details.');

    await percy.snapshot();
  });

  test('show no advisory data when none exist', async ({ page, msw }) => {
    let crate = await msw.db.crate.create({ name: 'safe-crate' });
    await msw.db.version.create({ crate, num: '1.0.0' });

    await msw.worker.use(
      http.get('https://rustsec.org/packages/:crateId.json', () => HttpResponse.text('not found', { status: 404 })),
    );

    await page.goto('/crates/safe-crate/security');

    await expect(page.locator('[data-no-advisories]')).toBeVisible();
    await expect(page.locator('[data-no-advisories]')).toHaveText('No advisories found for this crate.');
  });

  test('handles errors gracefully', async ({ page, msw }) => {
    let crate = await msw.db.crate.create({ name: 'error-crate' });
    await msw.db.version.create({ crate, num: '1.0.0' });

    await msw.worker.use(
      http.get('https://rustsec.org/packages/:crateId.json', () =>
        HttpResponse.text('Internal Server Error', { status: 500 }),
      ),
    );

    await page.goto('/crates/error-crate/security');

    // When there's an error, the route redirects to the catch-all error page
    await expect(page).toHaveURL('/crates/error-crate/security');
    await expect(page.locator('[data-test-404-page]')).toBeVisible();
    await expect(page.locator('[data-test-title]')).toHaveText('error-crate: Failed to load advisories');
    await expect(page.locator('[data-test-try-again]')).toBeVisible();
  });

  test('properly escapes HTML in advisory details', async ({ page, msw }) => {
    let crate = await msw.db.crate.create({ name: 'xss-test' });
    await msw.db.version.create({ crate, num: '1.0.0' });

    let advisories = [
      {
        id: 'TEST-XSS',
        summary: 'Advisory with XSS attempt',
        details: 'This advisory contains <script>alert("XSS")</script> and should be escaped.',
      },
    ];

    await msw.worker.use(http.get('https://rustsec.org/packages/:crateId.json', () => HttpResponse.json(advisories)));

    await page.goto('/crates/xss-test/security');

    await expect(page.locator('[data-test-list] li')).toHaveCount(1);

    let advisory = page.locator('[data-test-list] li').first();
    await expect(advisory.locator('h3 a')).toHaveText('TEST-XSS');
    await expect(advisory.locator('h3')).toContainText('Advisory with XSS attempt');

    // Verify the script tag is escaped and not executed
    expect(await advisory.locator('p').innerHTML()).toBe(
      'This advisory contains &lt;script&gt;alert("XSS")&lt;/script&gt; and should be escaped.',
    );
  });
});
