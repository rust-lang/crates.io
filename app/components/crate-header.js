import { service } from '@ember/service';
import Component from '@glimmer/component';

import { task } from 'ember-concurrency';
import { alias } from 'macro-decorators';

export default class CrateHeader extends Component {
  @service session;

  @alias('loadKeywordsTask.last.value') keywords;

  constructor() {
    super(...arguments);

    this.loadKeywordsTask.perform().catch(() => {
      // ignore all errors and just don't display keywords if the request fails
    });
  }

  get isOwner() {
    let userId = this.session.currentUser?.id;
    return this.args.crate?.hasOwnerUser(userId) ?? false;
  }

  loadKeywordsTask = task(async () => {
    return (await this.args.crate?.keywords) ?? [];
  });
}
