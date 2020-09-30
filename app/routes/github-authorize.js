import Route from '@ember/routing/route';

import window from 'ember-window-mock';
import fetch from 'fetch';

/**
 * This route will be called from the GitHub OAuth flow once the user has
 * accepted or rejected the data access permissions. It will forward the
 * temporary `code` received from the GitHub API to our own API server which
 * will exchange it for an access token.
 *
 * After the exchange the API will return an API token and the user information.
 * The result will be stored and the popup window closed. The `/login` route
 * will then continue to evaluate the response.
 *
 * @see https://developer.github.com/v3/oauth/#github-redirects-back-to-your-site
 * @see `/login` route
 */
export default Route.extend({
  async beforeModel(transition) {
    try {
      let { code, state } = transition.to.queryParams;
      let resp = await fetch(`/api/private/session/authorize?code=${code}&state=${state}`);
      let json = await resp.json();
      if (window.opener) {
        window.opener.postMessage({ ok: resp.ok, data: json }, window.location.origin);
      }
    } catch (d) {
      if (window.opener) {
        window.opener.postMessage({ ok: false, data: d }, window.location.origin);
      }
    } finally {
      window.close();
    }
  },
});
