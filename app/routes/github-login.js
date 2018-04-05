import Route from '@ember/routing/route';
import { inject as service } from '@ember/service';

/**
 * Calling this route will query the `/authorize_url` API endpoint
 * and redirect to the received URL to initiate the OAuth flow.
 *
 * Example URL:
 * https://github.com/login/oauth/authorize?client_id=...&state=...&scope=read%3Aorg
 *
 * Once the user has allowed the OAuth flow access the page will redirect him
 * to the `github-authorize` route of this application.
 *
 * @see https://developer.github.com/v3/oauth/#redirect-users-to-request-github-access
 * @see `github-authorize` route
 */
export default Route.extend({

    ajax: service(),

    async beforeModel() {
        let url = await this.get('ajax').request(`/authorize_url`);
        window.location = url.url;
    },
});
