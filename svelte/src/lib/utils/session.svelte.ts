import type { operations } from '@crates-io/api-client';
import type { NotificationsContext } from '$lib/notifications.svelte';

import { createContext } from 'svelte';
import { resolve } from '$app/paths';
import { createClient } from '@crates-io/api-client';

import * as localStorage from './local-storage';

const LOGIN_KEY = 'isLoggedIn';

const POPUP_FEATURES = [
  'width=1000',
  'height=450',
  'toolbar=0',
  'scrollbars=1',
  'status=1',
  'resizable=1',
  'location=1',
  'menuBar=0',
].join(',');

type ApiClient = ReturnType<typeof createClient>;

export type AuthenticatedUser =
  operations['get_authenticated_user']['responses']['200']['content']['application/json']['user'];

export function isLoggedIn(): boolean {
  return localStorage.getItem(LOGIN_KEY) === '1';
}

export async function loadUser(client: ApiClient): Promise<AuthenticatedUser | null> {
  if (!isLoggedIn()) {
    return null;
  }

  try {
    let response = await client.GET('/api/v1/me');
    if (response.error) {
      return null;
    }
    return response.data.user;
  } catch {
    return null;
  }
}

/**
 * Waits for either a postMessage from the OAuth popup or for the popup
 * to be closed by the user. Returns the OAuth callback parameters if
 * the message is received, or `{ closed: true }` if the popup was closed.
 */
function waitForOAuthCallback(popup: Window): Promise<{ code: string; state: string } | { closed: true }> {
  return new Promise(resolve => {
    let interval: ReturnType<typeof setInterval>;

    function onMessage(event: MessageEvent) {
      if (event.origin !== window.location.origin || !event.data) return;

      let { code, state } = event.data;
      if (!code || !state) return;

      cleanup();
      resolve({ code, state });
    }

    function cleanup() {
      window.removeEventListener('message', onMessage);
      clearInterval(interval);
    }

    window.addEventListener('message', onMessage);

    interval = setInterval(() => {
      if (popup.closed) {
        cleanup();
        resolve({ closed: true });
      }
    }, 10);
  });
}

export class SessionState {
  state: 'checking' | 'logged-out' | 'logging-in' | 'logged-in' | 'logging-out' = $state('checking');
  currentUser: AuthenticatedUser | null = $state(null);

  /**
   * Resolves once the initial user authentication check completes
   * and `currentUser` has been set via `setUser()`. Layouts that
   * require an authenticated user can `{#await}` this to avoid
   * rendering before `currentUser` is available.
   */
  initialPromise: Promise<void> | null = null;

  // TODO: implement `ownedCrates` (loaded from the `/api/v1/me` response)
  // TODO: implement sudo mode (`sudoEnabledUntil`, `isSudoEnabled`, `setSudo()`)
  //   Sudo mode enables admin actions for a limited duration. The expiry
  //   timestamp is persisted in localStorage under the 'sudo' key so it
  //   survives page reloads. On user load, the stored expiry is checked
  //   and sudo mode is restored if still valid.
  // TODO: implement saved transition (redirect to originally requested page after login)
  // TODO: integrate with Sentry (`sentry.setUser({ id })` after loading user)

  #client: ApiClient;
  #notifications?: NotificationsContext;

  constructor(client: ApiClient, notifications?: NotificationsContext) {
    this.#client = client;
    this.#notifications = notifications;
  }

  setUser(user: AuthenticatedUser | null): void {
    this.state = user ? 'logged-in' : 'logged-out';
    this.currentUser = user;
  }

  /**
   * Opens a popup window and initiates the GitHub OAuth flow.
   *
   * The popup navigates to `/github-auth-loading.html`, which fetches the
   * OAuth URL from `/api/private/session/begin` and redirects to GitHub.
   *
   * Example URL:
   * https://github.com/login/oauth/authorize?client_id=...&state=...&scope=read%3Aorg
   *
   * Once the user has allowed the OAuth flow, GitHub redirects back to
   * this application, and the popup sends a postMessage with the OAuth
   * callback parameters.
   *
   * Using a same-origin loading page avoids the opaque origin issue that
   * occurs with `document.write()` on a blank popup, which causes GitHub
   * to reject the OAuth POST with HTTP 422.
   *
   * @see https://developer.github.com/v3/oauth/#redirect-users-to-request-github-access
   * @see https://github.com/rust-lang/crates.io/discussions/4320
   */
  async login(): Promise<void> {
    let popup = window.open('/github-auth-loading.html', '_blank', POPUP_FEATURES);
    if (!popup) {
      return;
    }

    this.state = 'logging-in';

    let result = await waitForOAuthCallback(popup);
    if ('closed' in result) {
      this.#notifications?.warning('Login was canceled because the popup window was closed.');
      this.state = 'logged-out';
      return;
    }

    popup.close();

    let { code, state } = result;
    let { data, error } = await this.#client.GET('/api/private/session/authorize', {
      params: { query: { code, state } },
    });

    if (!data) {
      let detail = (error as unknown as { errors?: { detail?: string }[] })?.errors?.[0]?.detail;
      this.#notifications?.error(detail ? `Failed to log in: ${detail}` : 'Failed to log in');
      this.state = 'logged-out';
      return;
    }

    localStorage.setItem(LOGIN_KEY, '1');

    let user = await loadUser(this.#client);
    this.currentUser = user;
    this.state = user ? 'logged-in' : 'logged-out';

    // TODO: perform the originally saved transition, if it exists
  }

  async logout(): Promise<void> {
    this.state = 'logging-out';

    try {
      await this.#client.DELETE('/api/private/session');
    } finally {
      localStorage.removeItem(LOGIN_KEY);

      // Full page navigation to ensure all in-memory state is cleared on logout.
      window.location.assign(resolve('/'));
    }
  }
}

export const [getSession, setSession] = createContext<SessionState>();
