import { service } from '@ember/service';

import AuthenticatedRoute from '../-authenticated-route';

export default class RebuildDocsRoute extends AuthenticatedRoute {
  @service router;
  @service session;
  @service store;

  async model(params) {
    // Get the crate from parent route
    let crate = this.modelFor('crate');

    // Load the specific version
    let version = await this.store.queryRecord('version', {
      name: crate.id,
      num: params.version_num,
    });

    return { crate, version };
  }

  async afterModel(model, transition) {
    let user = this.session.currentUser;
    let owners = await model.crate.owner_user;
    let isOwner = owners.some(owner => owner.id === user.id);
    if (!isOwner) {
      this.router.replaceWith('catch-all', {
        transition,
        title: 'This page is only accessible by crate owners',
      });
    }
  }
}
