import { service } from '@ember/service';

import AuthenticatedRoute from '../-authenticated-route';

export default class SettingsRoute extends AuthenticatedRoute {
  @service router;
  @service session;

  async beforeModel(transition) {
    await super.beforeModel(...arguments);

    let user = this.session.currentUser;
    let owners = await this.modelFor('crate').owner_user;
    let isOwner = owners.some(owner => owner.id === user.id);
    if (!isOwner) {
      this.router.replaceWith('catch-all', {
        transition,
        title: 'This page is only accessible by crate owners',
      });
    }
  }

  setupController(controller) {
    super.setupController(...arguments);
    let crate = this.modelFor('crate');
    controller.set('crate', crate);
  }
}
