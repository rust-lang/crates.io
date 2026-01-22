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
        aliases: ['CVE-2024-12345', 'GHSA-abcd-1234-efgh'],
        severity: [{ type: 'CVSS_V3', score: 'CVSS:3.1/AV:N/AC:L/PR:N/UI:N/S:U/C:N/I:N/A:H' }],
        affected: [
          {
            ranges: [
              {
                type: 'SEMVER',
                events: [{ introduced: '0.0.0-0' }, { fixed: '0.7.46' }, { introduced: '0.8.0' }, { fixed: '0.8.13' }],
              },
            ],
          },
        ],
      },
      {
        id: 'TEST-002',
        summary: 'Second test advisory',
        details: 'This is the second test advisory with more details.',
      },
    ];

    await msw.worker.use(http.get('https://rustsec.org/packages/:crateId.json', () => HttpResponse.json(advisories)));

    await page.goto('/crates/foo/security');

    await expect(page.locator('[data-test-list] > li')).toHaveCount(2);

    // Check first advisory
    let advisory1 = page.locator('[data-test-list] > li').nth(0);
    await expect(advisory1.locator('h3 a')).toHaveAttribute('href', 'https://rustsec.org/advisories/TEST-001.html');
    await expect(advisory1.locator('h3 a')).toHaveText('TEST-001');
    await expect(advisory1.locator('h3')).toContainText('First test advisory');
    expect(await advisory1.locator('p').innerHTML()).toBe(
      'This is the first test advisory with <strong>markdown</strong> support.',
    );
    // Check version ranges are displayed
    await expect(advisory1.locator('[data-test-affected-versions]')).toBeVisible();
    await expect(advisory1.locator('[data-test-affected-versions]')).toContainText('Affected versions:');
    await expect(advisory1.locator('[data-test-affected-versions]')).toContainText('<0.7.46; >=0.8.0, <0.8.13');

    // Check aliases are displayed with correct links
    await expect(advisory1.locator('[data-test-aliases]')).toBeVisible();
    await expect(advisory1.locator('[data-test-aliases]')).toContainText('Aliases:');
    await expect(advisory1.locator('[data-test-aliases] li')).toHaveCount(2);
    await expect(advisory1.locator('[data-test-aliases] li').nth(0).locator('a')).toHaveText('CVE-2024-12345');
    await expect(advisory1.locator('[data-test-aliases] li').nth(0).locator('a')).toHaveAttribute(
      'href',
      'https://nvd.nist.gov/vuln/detail/CVE-2024-12345',
    );
    await expect(advisory1.locator('[data-test-aliases] li').nth(1).locator('a')).toHaveText('GHSA-abcd-1234-efgh');
    await expect(advisory1.locator('[data-test-aliases] li').nth(1).locator('a')).toHaveAttribute(
      'href',
      'https://github.com/advisories/GHSA-abcd-1234-efgh',
    );

    // Check CVSS is displayed with link to calculator
    await expect(advisory1.locator('[data-test-cvss]')).toBeVisible();
    await expect(advisory1.locator('[data-test-cvss]')).toContainText('CVSS:');
    await expect(advisory1.locator('[data-test-cvss] a')).toHaveText('CVSS:3.1/AV:N/AC:L/PR:N/UI:N/S:U/C:N/I:N/A:H');
    await expect(advisory1.locator('[data-test-cvss] a')).toHaveAttribute(
      'href',
      'https://www.first.org/cvss/calculator/3.1#CVSS:3.1/AV:N/AC:L/PR:N/UI:N/S:U/C:N/I:N/A:H',
    );

    // Check second advisory (without version ranges, aliases, or CVSS)
    let advisory2 = page.locator('[data-test-list] > li').nth(1);
    await expect(advisory2.locator('h3 a')).toHaveAttribute('href', 'https://rustsec.org/advisories/TEST-002.html');
    await expect(advisory2.locator('h3 a')).toHaveText('TEST-002');
    await expect(advisory2.locator('h3')).toContainText('Second test advisory');
    expect(await advisory2.locator('p').innerHTML()).toBe('This is the second test advisory with more details.');
    // Verify no version ranges, aliases, or CVSS section for advisory without that data
    await expect(advisory2.locator('[data-test-affected-versions]')).not.toBeVisible();
    await expect(advisory2.locator('[data-test-aliases]')).not.toBeVisible();
    await expect(advisory2.locator('[data-test-cvss]')).not.toBeVisible();

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

    await expect(page.locator('[data-test-list] > li')).toHaveCount(1);

    let advisory = page.locator('[data-test-list] > li').first();
    await expect(advisory.locator('h3 a')).toHaveText('TEST-XSS');
    await expect(advisory.locator('h3')).toContainText('Advisory with XSS attempt');

    // Verify the script tag is escaped and not executed
    expect(await advisory.locator('p').innerHTML()).toBe(
      'This advisory contains &lt;script&gt;alert("XSS")&lt;/script&gt; and should be escaped.',
    );
  });

  test('filters out unmaintained advisories', async ({ page, msw }) => {
    let crate = await msw.db.crate.create({ name: 'unmaintained-test' });
    await msw.db.version.create({ crate, num: '1.0.0' });

    let advisories = [
      {
        id: 'TEST-VULN',
        summary: 'Actual security vulnerability',
        details: 'This is a real security issue.',
      },
      {
        id: 'TEST-UNMAINTAINED',
        summary: 'Package is unmaintained',
        details: 'This package is no longer maintained.',
        affected: [
          {
            database_specific: {
              informational: 'unmaintained',
            },
          },
        ],
      },
      {
        id: 'TEST-ANOTHER',
        summary: 'Another vulnerability',
        details: 'Another real security issue.',
      },
    ];

    await msw.worker.use(http.get('https://rustsec.org/packages/:crateId.json', () => HttpResponse.json(advisories)));
    await page.goto('/crates/unmaintained-test/security');

    // Should only show 2 advisories (the unmaintained one should be filtered out)
    await expect(page.locator('[data-test-list] > li')).toHaveCount(2);

    // Verify the unmaintained advisory is not shown
    await expect(page.locator('[data-test-list]')).not.toContainText('TEST-UNMAINTAINED');
    await expect(page.locator('[data-test-list]')).not.toContainText('Package is unmaintained');

    // Verify the actual vulnerabilities are shown
    await expect(page.locator('[data-test-list]')).toContainText('TEST-VULN');
    await expect(page.locator('[data-test-list]')).toContainText('TEST-ANOTHER');
  });

  test('filters out withdrawn advisories', async ({ page, msw }) => {
    let crate = await msw.db.crate.create({ name: 'withdrawn-test' });
    await msw.db.version.create({ crate, num: '1.0.0' });

    let advisories = [
      {
        id: 'TEST-ACTIVE',
        summary: 'Active security vulnerability',
        details: 'This is an active security issue.',
      },
      {
        id: 'TEST-WITHDRAWN',
        summary: 'Withdrawn advisory',
        details: 'This advisory was withdrawn after circumstances changed.',
        withdrawn: '2025-02-22T12:00:00Z',
      },
    ];

    await msw.worker.use(http.get('https://rustsec.org/packages/:crateId.json', () => HttpResponse.json(advisories)));
    await page.goto('/crates/withdrawn-test/security');

    // Should only show 1 advisory (the withdrawn one should be filtered out)
    await expect(page.locator('[data-test-list] > li')).toHaveCount(1);

    // Verify the withdrawn advisory is not shown
    await expect(page.locator('[data-test-list]')).not.toContainText('TEST-WITHDRAWN');
    await expect(page.locator('[data-test-list]')).not.toContainText('Withdrawn advisory');

    // Verify the active vulnerability is shown
    await expect(page.locator('[data-test-list]')).toContainText('TEST-ACTIVE');
    await expect(page.locator('[data-test-list]')).toContainText('Active security vulnerability');
  });

  test('prefers CVSS V4 over V3 when both are present', async ({ page, msw }) => {
    let crate = await msw.db.crate.create({ name: 'cvss-test' });
    await msw.db.version.create({ crate, num: '1.0.0' });

    let advisories = [
      {
        id: 'TEST-CVSS-V4',
        summary: 'Advisory with CVSS V4',
        details: 'This advisory has both V3 and V4 CVSS scores.',
        severity: [
          { type: 'CVSS_V3', score: 'CVSS:3.1/AV:N/AC:L/PR:N/UI:N/S:U/C:N/I:N/A:H' },
          { type: 'CVSS_V4', score: 'CVSS:4.0/AV:N/AC:L/AT:N/PR:N/UI:N/VC:N/VI:N/VA:H/SC:N/SI:N/SA:N' },
        ],
      },
    ];

    await msw.worker.use(http.get('https://rustsec.org/packages/:crateId.json', () => HttpResponse.json(advisories)));
    await page.goto('/crates/cvss-test/security');

    let advisory = page.locator('[data-test-list] li').first();
    await expect(advisory.locator('[data-test-cvss]')).toBeVisible();
    // Should show V4, not V3
    await expect(advisory.locator('[data-test-cvss] a')).toHaveText(
      'CVSS:4.0/AV:N/AC:L/AT:N/PR:N/UI:N/VC:N/VI:N/VA:H/SC:N/SI:N/SA:N',
    );
    await expect(advisory.locator('[data-test-cvss] a')).toHaveAttribute(
      'href',
      'https://www.first.org/cvss/calculator/4.0#CVSS:4.0/AV:N/AC:L/AT:N/PR:N/UI:N/VC:N/VI:N/VA:H/SC:N/SI:N/SA:N',
    );
  });

  test('supports CVSS 3.0 vectors', async ({ page, msw }) => {
    let crate = await msw.db.crate.create({ name: 'cvss30-test' });
    await msw.db.version.create({ crate, num: '1.0.0' });

    let advisories = [
      {
        id: 'TEST-CVSS-V30',
        summary: 'Advisory with CVSS 3.0',
        details: 'This advisory has a CVSS 3.0 score.',
        severity: [{ type: 'CVSS_V3', score: 'CVSS:3.0/AV:N/AC:L/PR:N/UI:N/S:U/C:H/I:H/A:H' }],
      },
    ];

    await msw.worker.use(http.get('https://rustsec.org/packages/:crateId.json', () => HttpResponse.json(advisories)));
    await page.goto('/crates/cvss30-test/security');

    let advisory = page.locator('[data-test-list] > li').first();
    await expect(advisory.locator('[data-test-cvss]')).toBeVisible();
    await expect(advisory.locator('[data-test-cvss] a')).toHaveText('CVSS:3.0/AV:N/AC:L/PR:N/UI:N/S:U/C:H/I:H/A:H');
    await expect(advisory.locator('[data-test-cvss] a')).toHaveAttribute(
      'href',
      'https://www.first.org/cvss/calculator/3.0#CVSS:3.0/AV:N/AC:L/PR:N/UI:N/S:U/C:H/I:H/A:H',
    );
  });
});
