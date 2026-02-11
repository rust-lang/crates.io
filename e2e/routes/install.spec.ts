import { expect, test } from '@/e2e/helper';

test.describe('Route | install', { tag: '@routes' }, () => {
  test('redirects to the Cargo installation guide', async ({ page }) => {
    let url = 'https://doc.rust-lang.org/cargo/getting-started/installation.html';

    await page.route('https://doc.rust-lang.org/**', route => route.fulfill({ body: 'redirected' }));

    await page.goto('/install');
    await expect(page).toHaveURL(url);
  });
});
