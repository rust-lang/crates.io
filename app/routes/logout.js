import { run } from '@ember/runloop';
import Route from '@ember/routing/route';
import { inject as service } from '@ember/service';
import ajax from 'ember-fetch/ajax';

export default class LogoutRoute extends Route {
  @service session;

  async activate() {
    await ajax(`/api/private/session`, { method: 'DELETE' });
    run(() => {
      this.session.logoutUser();
      this.transitionTo('index');
    });
  }
}
