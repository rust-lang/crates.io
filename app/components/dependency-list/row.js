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

  get featuresDescription() {
    let { default_features: defaultFeatures, features } = this.args.dependency;
    let numFeatures = features.length;

    if (numFeatures !== 0) {
      return defaultFeatures
        ? `${numFeatures} extra feature${numFeatures > 1 ? 's' : ''}`
        : `only ${numFeatures} feature${numFeatures > 1 ? 's' : ''}`;
    } else if (!defaultFeatures) {
      return 'no default features';
    }
  }

  @task *loadCrateTask() {
    let { dependency } = this.args;
    return yield this.store.findRecord('crate', dependency.crate_id);
  }
}
