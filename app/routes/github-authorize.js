import Route from '@ember/routing/route';
import fetch from 'fetch';
import { serializeQueryParams } from 'ember-fetch/utils/serialize-query-params';

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
            let queryParams = serializeQueryParams(transition.queryParams);
            let resp = await fetch(`/authorize?${queryParams}`);
            let json = await resp.json();
            let item = JSON.stringify({ ok: resp.ok, data: json });
            if (window.opener) {
                window.opener.github_response = item;
            }
        } catch (d) {
            let item = JSON.stringify({ ok: false, data: d });
            if (window.opener) {
                window.opener.github_response = item;
            }
        } finally {
            window.close();
        }
    },
});
