import { service } from '@ember/service';

import AuthenticatedRoute from '../-authenticated-route';

export default class SettingsRoute extends AuthenticatedRoute {
  @service router;
  @service session;
  @service store;

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

  async model() {
    let crate = this.modelFor('crate');

    let githubConfigs = await this.store.query('trustpub-github-config', { crate: crate.name });

    return { crate, githubConfigs };
  }

  setupController(controller, { crate, githubConfigs }) {
    super.setupController(...arguments);

    controller.set('crate', crate);
    controller.set('githubConfigs', githubConfigs);
  }
}
