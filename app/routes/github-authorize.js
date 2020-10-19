import Route from '@ember/routing/route';

import window from 'ember-window-mock';

/**
 * This route will be called from the GitHub OAuth flow once the user has
 * accepted or rejected the data access permissions. It will forward the
 * temporary `code` received from the GitHub API to the parent window which
 * will send it to our own API server which will then exchange it for an
 * access token.
 *
 * After the exchange the API will return an API token and the user information.
 * The result will be stored and the popup window closed. The `/login` route
 * will then continue to evaluate the response.
 *
 * @see https://developer.github.com/v3/oauth/#github-redirects-back-to-your-site
 * @see `/login` route
 */
export default class GithubAuthorizeRoute extends Route {
  beforeModel(transition) {
    let { code, state } = transition.to.queryParams;
    window.opener?.postMessage({ code, state }, window.location.origin);
  }
}
