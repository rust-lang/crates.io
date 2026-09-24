import type { Page } from '@playwright/test';

export const MOCK_CODE = '901dd10e07c7e9fa1cd5';
export const MOCK_STATE = 'fYcUY3FMdUUz00FC7vLT7A';

/** Mocks GitHub OAuth redirects for login and signup acceptance tests. */
export async function setupGitHubOAuthRoutes(page: Page) {
  // Context routes also apply to the OAuth popup.
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

  await page.context().route('https://github.com/login/oauth/authorize*', route => {
    let url = new URL(route.request().url());
    let state = url.searchParams.get('state');
    let redirectUrl = new URL(`/github-redirect.html?code=${MOCK_CODE}&state=${state}`, page.url());
    route.fulfill({ status: 302, headers: { Location: redirectUrl.toString() } });
  });
}
