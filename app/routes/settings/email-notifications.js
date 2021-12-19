import { inject as service } from '@ember/service';

import AuthenticatedRoute from '../-authenticated-route';

export default class EmailNotificationsSettingsRoute extends AuthenticatedRoute {
  @service store;

  async model() {
    let { ownedCrates, currentUser: user } = this.session;

    if (!ownedCrates) {
      await this.session.fetchUser();
      ({ ownedCrates } = this.session);
    }

    return { user, ownedCrates };
  }

  setupController(controller) {
    super.setupController(...arguments);

    controller.setProperties({
      emailNotificationsSuccess: false,
      emailNotificationsError: false,
    });
  }
}
