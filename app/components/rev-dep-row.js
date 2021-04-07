import { action } from '@ember/object';
import { inject as service } from '@ember/service';
import Component from '@glimmer/component';
import { tracked } from '@glimmer/tracking';

import { task } from 'ember-concurrency';

export default class VersionRow extends Component {
  @service store;

  @tracked focused = false;

  @action setFocused(value) {
    this.focused = value;
  }

  constructor() {
    super(...arguments);

    this.loadCrateTask.perform().catch(() => {
      // ignore all errors and just don't display a description if the request fails
    });
  }

  get description() {
    return this.loadCrateTask.lastSuccessful?.value?.description;
  }

  @task *loadCrateTask() {
    let { dependency } = this.args;
    return yield this.store.findRecord('crate', dependency.version.crateName);
  }
}
