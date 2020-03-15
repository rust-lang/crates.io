import Route from '@ember/routing/route';
import { action } from '@ember/object';

import AuthenticatedRoute from '../../mixins/authenticated-route';

export default class MeIndexRoute extends Route.extend(AuthenticatedRoute) {
  @action
  willTransition() {
    this.controller
      .setProperties({
        emailNotificationsSuccess: false,
        emailNotificationsError: false,
      })
      .clear();
  }

  model() {
    return {
      user: this.get('session.currentUser'),
      ownedCrates: this.get('session.ownedCrates'),
      api_tokens: this.store.findAll('api-token'),
    };
  }
}
