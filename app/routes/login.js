import Route from '@ember/routing/route';
import { inject as service } from '@ember/service';

import window from 'ember-window-mock';

import * as localStorage from '../utils/local-storage';

/**
 * This route will open a popup window directed at the `github-login` route.
 * After the window has opened it will wait for the window to close and
 * then evaluate whether the OAuth flow was successful.
 *
 * @see `github-authorize` route
 */
export default Route.extend({
  notifications: service(),
  session: service(),

  beforeModel(transition) {
    localStorage.removeItem('github_response');

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
        this.notifications.error(`Failed to log in: ${data.errors[0].detail}`);
        return;
      } else if (!response.ok) {
        this.notifications.error('Failed to log in');
        return;
      }

      this.session.login();
    }, 200);

    transition.abort();
  },
});
