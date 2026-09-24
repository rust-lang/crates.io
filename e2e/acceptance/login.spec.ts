import { MOCK_CODE, MOCK_STATE, setupGitHubOAuthRoutes } from '@/e2e/fixtures/github-oauth';
import { expect, test } from '@/e2e/helper';
import { serializeUser } from '@crates-io/msw/serializers/user';
import { http } from '@crates-io/msw/utils/openapi-http';

test.describe('Acceptance | Login', { tag: '@acceptance' }, () => {
  test('successful login', async ({ page, msw }) => {
    await setupGitHubOAuthRoutes(page);

    let user = await msw.db.user.create({ name: 'John Doe' });

    msw.worker.use(
      http.post('/api/private/session/authorize', async ({ request, response }) => {
        let body = await request.json();
        expect(Object.keys(body)).toEqual(['code', 'state']);
        expect(body.code).toBe(MOCK_CODE);
        expect(body.state).toBe(MOCK_STATE);

        await msw.db.mswSession.create({ user });
        return response(200).json({
          status: 'signed_in',
          user: serializeUser(user, { removePrivateData: false }),
          owned_crates: [],
        });
      }),
    );

    await page.goto('/');
    await page.click('[data-test-login-button]');
    await expect(page.locator('[data-test-user-menu] [data-test-toggle]')).toHaveText('John Doe');
  });

  test('failed login', async ({ page, msw }) => {
    await setupGitHubOAuthRoutes(page);

    msw.worker.use(
      http.post('/api/private/session/authorize', ({ response }) =>
        response.untyped(Response.json({ errors: [{ detail: 'Forbidden' }] }, { status: 403 })),
      ),
    );

    await page.goto('/');
    await page.click('[data-test-login-button]');
    await expect(page.locator('[data-test-notification-message]')).toHaveText('Failed to log in: Forbidden');
  });

  test('redirects to originally requested page after login', async ({ page, msw }) => {
    await setupGitHubOAuthRoutes(page);

    let user = await msw.db.user.create({ name: 'John Doe' });

    msw.worker.use(
      http.post('/api/private/session/authorize', async ({ request, response }) => {
        let body = await request.json();
        expect(Object.keys(body)).toEqual(['code', 'state']);
        expect(body.code).toBe(MOCK_CODE);
        expect(body.state).toBe(MOCK_STATE);

        await msw.db.mswSession.create({ user });
        return response(200).json({
          status: 'signed_in',
          user: serializeUser(user, { removePrivateData: false }),
          owned_crates: [],
        });
      }),
    );

    // Navigate to a protected page while logged out
    await page.goto('/settings/profile');
    await expect(page.locator('[data-test-title]')).toHaveText('This page requires authentication');

    // Click the login button on the error page
    await page.click('[data-test-login]');

    // After login, the user should be redirected to the originally requested page
    await expect(page.locator('[data-test-user-menu] [data-test-toggle]')).toHaveText('John Doe');
    await expect(page).toHaveURL('/settings/profile');
    await expect(page.locator('[data-test-page-header]')).toHaveText('Account Settings');
  });

  test('login canceled when popup is closed', async ({ page }) => {
    await setupGitHubOAuthRoutes(page);

    // Override the GitHub OAuth route to hang instead of redirecting
    await page.context().route('https://github.com/login/oauth/authorize*', () => {
      // Don't fulfill - let the request hang
    });

    await page.goto('/');

    let popupPromise = page.waitForEvent('popup');

    await page.click('[data-test-login-button]');

    let popup = await popupPromise;
    await popup.close();

    await expect(page.locator('[data-test-notification-message]')).toHaveText(
      'Login was canceled because the popup window was closed.',
    );
  });
});
