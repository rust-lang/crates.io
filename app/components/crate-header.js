import { inject as service } from '@ember/service';
import Component from '@glimmer/component';

import { task } from 'ember-concurrency';
import { alias } from 'macro-decorators';

export default class CrateHeader extends Component {
  @service session;

  @alias('loadKeywordsTask.last.value') keywords;
  @alias('loadOwnerUserTask.last.value') ownerUser;

  constructor() {
    super(...arguments);

    this.loadKeywordsTask.perform().catch(() => {
      // ignore all errors and just don't display keywords if the request fails
    });
    this.loadOwnerUserTask.perform().catch(() => {
      // ignore all errors and just don't display settings if the request fails
    });
  }

  get isOwner() {
    let ownerUser = this.ownerUser ?? [];
    let currentUserId = this.session.currentUser?.id;
    return ownerUser.some(({ id }) => id === currentUserId);
  }

  loadKeywordsTask = task(async () => {
    return (await this.args.crate?.keywords) ?? [];
  });

  loadOwnerUserTask = task(async () => {
    return (await this.args.crate?.owner_user) ?? [];
  });
}
