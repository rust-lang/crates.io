import { expect, test } from '@/e2e/helper';

test.describe('Acceptance | crate page | native replacement', { tag: '@acceptance' }, () => {
  test('shows the banner for a crate superseded by std', async ({ page, msw }) => {
    let crate = await msw.db.crate.create({ name: 'lazy_static' });
    await msw.db.version.create({ crate, num: '1.4.0' });

    await page.goto('/crates/lazy_static');

    let banner = page.locator('[data-test-native-replacement-banner]');
    await expect(banner).toBeVisible();
    await expect(banner).toContainText('You might not need this dependency.');
    await expect(banner).toContainText('std::sync::LazyLock');
    await expect(banner.getByRole('link', { name: 'Learn more' })).toHaveAttribute(
      'href',
      'https://doc.rust-lang.org/std/sync/struct.LazyLock.html',
    );
  });

  test('does not show the banner for a crate without a replacement', async ({ page, msw }) => {
    let crate = await msw.db.crate.create({ name: 'nanomsg' });
    await msw.db.version.create({ crate, num: '0.6.1' });

    await page.goto('/crates/nanomsg');

    await expect(page.locator('[data-test-heading] [data-test-crate-name]')).toHaveText('nanomsg');
    await expect(page.locator('[data-test-native-replacement-banner]')).toHaveCount(0);
  });
});
