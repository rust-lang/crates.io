import { test, expect } from '@/e2e/helper';
import { http, HttpResponse } from 'msw';

test.describe('Acceptance | Login', { tag: '@acceptance' }, () => {
  test('successful login', async ({ page, msw }) => {
    // mock `window.open()`
    await page.addInitScript(() => {
      globalThis.open = (url, target, features) => {
        globalThis.openKwargs = { url, target, features };
        return { document: { write() {}, close() {} }, close() {} } as ReturnType<(typeof globalThis)['open']>;
      };
    });

    msw.worker.use(
      http.get('/api/private/session/begin', () => HttpResponse.json({ url: 'url-to-github-including-state-secret' })),
      http.get('/api/private/session/authorize', ({ request }) => {
        let url = new URL(request.url);
        expect([...url.searchParams.keys()]).toEqual(['code', 'state']);
        expect(url.searchParams.get('code')).toBe('901dd10e07c7e9fa1cd5');
        expect(url.searchParams.get('state')).toBe('fYcUY3FMdUUz00FC7vLT7A');

        let user = msw.db.user.create();
        msw.db.mswSession.create({ user });
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
    await expect(page).toHaveURL('/');

    await page.click('[data-test-login-button]');
    await expect(page).toHaveURL('/');

    await page.waitForFunction(expect => globalThis.openKwargs.url === expect, '');
    await page.waitForFunction(expect => globalThis.openKwargs.target === expect, '_blank');
    await page.waitForFunction(
      expect => globalThis.openKwargs.features === expect,
      'width=1000,height=450,toolbar=0,scrollbars=1,status=1,resizable=1,location=1,menuBar=0',
    );

    // simulate the response from the `github-authorize` route
    let message = { code: '901dd10e07c7e9fa1cd5', state: 'fYcUY3FMdUUz00FC7vLT7A' };
    await page.evaluate(message => {
      window.postMessage(message, window.location.origin);
    }, message);

    await expect(page.locator('[data-test-user-menu] [data-test-toggle]')).toHaveText('John Doe');
  });

  test('failed login', async ({ page, msw }) => {
    // mock `window.open()`
    await page.addInitScript(() => {
      globalThis.open = (url, target, features) => {
        globalThis.openKwargs = { url, target, features };
        return { document: { write() {}, close() {} }, close() {} } as ReturnType<(typeof globalThis)['open']>;
      };
    });

    msw.worker.use(
      http.get('/api/private/session/begin', () => HttpResponse.json({ url: 'url-to-github-including-state-secret' })),
      http.get('/api/private/session/authorize', () =>
        HttpResponse.json({ errors: [{ detail: 'Forbidden' }] }, { status: 403 }),
      ),
    );

    await page.goto('/');
    await expect(page).toHaveURL('/');

    await page.click('[data-test-login-button]');
    await expect(page).toHaveURL('/');

    // wait for `window.open()` to be called
    await page.waitForFunction(() => !!globalThis.openKwargs);

    // simulate the response from the `github-authorize` route
    let message = { code: '901dd10e07c7e9fa1cd5', state: 'fYcUY3FMdUUz00FC7vLT7A' };
    await page.evaluate(message => {
      window.postMessage(message, window.location.origin);
    }, message);

    await expect(page.locator('[data-test-notification-message]')).toHaveText('Failed to log in: Forbidden');
  });
});
