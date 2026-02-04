import { expect, test } from '@/e2e/helper';
import type { Page } from '@playwright/test';
import { http, HttpResponse } from 'msw';

const MOCK_CODE = '901dd10e07c7e9fa1cd5';
const MOCK_STATE = 'fYcUY3FMdUUz00FC7vLT7A';

async function setupGitHubOAuthRoutes(page: Page) {
  // Intercept `/api/private/session/begin` at the context level (applies to popups too)
  await page.context().route('**/api/private/session/begin', route => {
    route.fulfill({
      status: 200,
      contentType: 'application/json',
      body: JSON.stringify({
        url: `https://github.com/login/oauth/authorize?client_id=test&state=${MOCK_STATE}&scope=read:org`,
        state: MOCK_STATE,
      }),
    });
  });

  // Intercept GitHub OAuth URL at the context level (applies to popups too)
  await page.context().route('https://github.com/login/oauth/authorize*', route => {
    let url = new URL(route.request().url());
    let state = url.searchParams.get('state');
    let redirectUrl = new URL(`/github-redirect.html?code=${MOCK_CODE}&state=${state}`, page.url());
    route.fulfill({ status: 302, headers: { Location: redirectUrl.toString() } });
  });
}

test.describe('Acceptance | Login', { tag: '@acceptance' }, () => {
  test('successful login', async ({ page, msw }) => {
    await setupGitHubOAuthRoutes(page);

    await msw.worker.use(
      http.get('/api/private/session/authorize', async ({ request }) => {
        let url = new URL(request.url);
        expect([...url.searchParams.keys()]).toEqual(['code', 'state']);
        expect(url.searchParams.get('code')).toBe(MOCK_CODE);
        expect(url.searchParams.get('state')).toBe(MOCK_STATE);

        let user = await msw.db.user.create({});
        await msw.db.mswSession.create({ user });
        return HttpResponse.json({ ok: true });
      }),

      http.get('/api/v1/me', () =>
        HttpResponse.json({
          user: {
            id: 42,
            login: 'johnnydee',
            name: 'John Doe',
            email: 'john@doe.name',
            avatar: 'https://avatars2.githubusercontent.com/u/12345?v=4',
            url: 'https://github.com/johnnydee',
          },
          owned_crates: [],
        }),
      ),
    );

    await page.goto('/');
    await page.click('[data-test-login-button]');
    await expect(page.locator('[data-test-user-menu] [data-test-toggle]')).toHaveText('John Doe');
  });

  test('failed login', async ({ page, msw }) => {
    await setupGitHubOAuthRoutes(page);

    await msw.worker.use(
      http.get('/api/private/session/authorize', () =>
        HttpResponse.json({ errors: [{ detail: 'Forbidden' }] }, { status: 403 }),
      ),
    );

    await page.goto('/');
    await page.click('[data-test-login-button]');
    await expect(page.locator('[data-test-notification-message]')).toHaveText('Failed to log in: Forbidden');
  });

  test('login canceled when popup is closed', async ({ page }) => {
    await setupGitHubOAuthRoutes(page);

    // Override the GitHub OAuth route to hang instead of redirecting
    await page.context().route('https://github.com/login/oauth/authorize*', () => {
      // Don't fulfill - let the request hang
    });

    await page.goto('/');

    const popupPromise = page.waitForEvent('popup');

    await page.click('[data-test-login-button]');

    const popup = await popupPromise;
    await popup.close();

    await expect(page.locator('[data-test-notification-message]')).toHaveText(
      'Login was canceled because the popup window was closed.',
    );
  });
});
