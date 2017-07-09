import Route from '@ember/routing/route';
import { inject as service } from '@ember/service';

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

    ajax: service(),

    beforeModel(transition) {
        return this.get('ajax').request(`/authorize`, { data: transition.queryParams }).then((d) => {
            let item = JSON.stringify({ ok: true, data: d });
            if (window.opener) {
                window.opener.github_response = item;
            }
        }).catch((d) => {
            let item = JSON.stringify({ ok: false, data: d });
            if (window.opener) {
                window.opener.github_response = item;
            }
        }).finally(() => {
            window.close();
        });
    },
});
