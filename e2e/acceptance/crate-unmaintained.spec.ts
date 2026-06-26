import { expect, test } from '@/e2e/helper';
import { http, HttpResponse } from 'msw';

const ADVISORY = 'RUSTSEC-2021-0139';

function unmaintainedAdvisory() {
  return [
    {
      id: ADVISORY,
      summary: 'nanomsg is unmaintained',
      details: 'The author has stated that this crate is no longer maintained.',
      affected: [{ ranges: [], database_specific: { informational: 'unmaintained' } }],
    },
  ];
}

test.describe('Acceptance | crate page | unmaintained', { tag: '@acceptance' }, () => {
  test('shows the banner for a crate marked unmaintained by RustSec', async ({ page, msw }) => {
    let crate = await msw.db.crate.create({ name: 'nanomsg' });
    await msw.db.version.create({ crate, num: '0.6.1' });

    msw.worker.use(
      http.get('https://rustsec.org/packages/:crateId.json', () => HttpResponse.json(unmaintainedAdvisory())),
    );

    await page.goto('/crates/nanomsg');

    let banner = page.locator('[data-test-unmaintained-banner]');
    await expect(banner).toBeVisible();
    await expect(banner).toContainText('This crate has been marked as unmaintained');

    let link = banner.getByRole('link', { name: ADVISORY });
    await expect(link).toHaveAttribute('href', `https://rustsec.org/advisories/${ADVISORY}.html`);
  });

  test('does not show the banner when there are no advisories', async ({ page, msw }) => {
    let crate = await msw.db.crate.create({ name: 'serde' });
    await msw.db.version.create({ crate, num: '1.0.0' });

    msw.worker.use(
      http.get('https://rustsec.org/packages/:crateId.json', () => HttpResponse.text('not found', { status: 404 })),
    );

    await page.goto('/crates/serde');

    await expect(page.locator('[data-test-heading] [data-test-crate-name]')).toHaveText('serde');
    await expect(page.locator('[data-test-unmaintained-banner]')).toHaveCount(0);
  });

  test('does not show the banner for an advisory with a patched version', async ({ page, msw }) => {
    let crate = await msw.db.crate.create({ name: 'patched' });
    await msw.db.version.create({ crate, num: '1.0.0' });

    let advisories = [
      {
        id: ADVISORY,
        summary: 'patched is unmaintained but has a fix',
        details: '',
        affected: [
          {
            ranges: [{ type: 'SEMVER', events: [{ introduced: '0.0.0-0' }, { fixed: '1.0.0' }] }],
            database_specific: { informational: 'unmaintained' },
          },
        ],
      },
    ];

    msw.worker.use(http.get('https://rustsec.org/packages/:crateId.json', () => HttpResponse.json(advisories)));

    await page.goto('/crates/patched');

    await expect(page.locator('[data-test-heading] [data-test-crate-name]')).toHaveText('patched');
    await expect(page.locator('[data-test-unmaintained-banner]')).toHaveCount(0);
  });
});
