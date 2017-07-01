import Ember from 'ember';

const { inject: { service } } = Ember;

/**
 * Calling this route will query the `/authorize_url` API endpoint
 * and redirect to the received URL to initiate the OAuth flow.
 *
 * Example URL:
 * https://github.com/login/oauth/authorize?client_id=...&state=...&scope=read%3Aorg
 *
 * Once the user has allowed the OAuth flow access the page will redirect him
 * to the `/github_authorize` route of this application.
 *
 * @see https://developer.github.com/v3/oauth/#redirect-users-to-request-github-access
 * @see `/github_authorize` route
 */
export default Ember.Route.extend({

    ajax: service(),

    beforeModel() {
        return this.get('ajax').request(`/authorize_url`).then((url) => {
            window.location = url.url;
        });
    },
});
