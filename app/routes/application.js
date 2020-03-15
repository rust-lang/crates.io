import Route from '@ember/routing/route';
import { inject as service } from '@ember/service';
import { action } from '@ember/object';

export default class ApplicationRoute extends Route {
  @service flashMessages;
  @service session;

  beforeModel() {
    this.session.loadUser();
  }

  @action
  didTransition() {
    this.flashMessages.step();
  }
}
