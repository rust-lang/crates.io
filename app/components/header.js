import { action } from '@ember/object';
import { inject as service } from '@ember/service';
import Component from '@glimmer/component';

import { task, waitForEvent } from 'ember-concurrency';
import window from 'ember-window-mock';

import ajax from '../utils/ajax';

export default class Header extends Component {
  @service header;
  @service notifications;
  @service router;
  @service session;

  @action
  search(event) {
    event.preventDefault();

    this.router.transitionTo('search', {
      queryParams: {
        q: this.header.searchValue,
        page: 1,
      },
    });
  }

  @action login() {
    this.loginTask.perform();
  }

  /**
   * This task will open a popup window directed at the `github-login` route.
   * After the window has opened it will wait for the window to send a message
   * back and then evaluate whether the OAuth flow was successful.
   *
   * @see `github-authorize` route
   */
  @task(function* () {
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

    let event = yield waitForEvent(window, 'message');
    if (event.origin !== window.location.origin || !event.data) {
      return;
    }

    let { data } = event.data;
    if (data && data.errors) {
      this.notifications.error(`Failed to log in: ${data.errors[0].detail}`);
      return;
    } else if (!event.data.ok) {
      this.notifications.error('Failed to log in');
      return;
    }

    this.session.login();
  })
  loginTask;

  @task(function* () {
    yield ajax(`/api/private/session`, { method: 'DELETE' });
    this.session.logoutUser();
    this.transitionTo('index');
  })
  logoutTask;
}
