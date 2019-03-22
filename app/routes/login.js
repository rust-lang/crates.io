import Route from '@ember/routing/route';
import { inject as service } from '@ember/service';

/**
 * This route will open a popup window directed at the `github-login` route.
 * After the window has opened it will wait for the window to close and
 * then evaluate whether the OAuth flow was successful.
 *
 * @see `github-authorize` route
 */
export default Route.extend({
    flashMessages: service(),
    session: service(),

    beforeModel(transition) {
        try {
            localStorage.removeItem('github_response');
        } catch (e) {
            // ignore error
        }

        window.github_response = undefined;
        let windowDimensions = [
            'width=1000',
            'height=450',
            'toolbar=0',
            'scrollbars=1',
            'status=1',
            'resizable=1',
            'location=1',
            'menuBar=0',
        ].join(',');

        let win = window.open('/github_login', 'Authorization', windowDimensions);
        if (!win) {
            return;
        }

        // For the life of me I cannot figure out how to do this other than
        // polling
        let oauthInterval = window.setInterval(() => {
            if (!win.closed) {
                return;
            }
            window.clearInterval(oauthInterval);
            let json = window.github_response;
            window.github_response = undefined;
            if (!json) {
                return;
            }

            let response = JSON.parse(json);
            if (!response) {
                return;
            }

            let { data } = response;
            if (data && data.errors) {
                let error = `Failed to log in: ${data.errors[0].detail}`;
                this.flashMessages.show(error);
                return;
            } else if (!response.ok) {
                this.flashMessages.show('Failed to log in');
                return;
            }

            let user = this.store.push(this.store.normalize('user', data.user));
            let transition = this.get('session.savedTransition');
            this.session.loginUser(user);
            if (transition) {
                transition.retry();
            }
        }, 200);

        transition.abort();
    },
});
