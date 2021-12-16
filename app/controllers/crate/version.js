import Controller from '@ember/controller';
import { inject as service } from '@ember/service';

import { task } from 'ember-concurrency';
import { alias } from 'macro-decorators';

export default class CrateVersionController extends Controller {
  @service session;

  get downloadsContext() {
    return this.requestedVersion ? this.currentVersion : this.crate;
  }

  @alias('downloadsContext.version_downloads.content') downloads;
  @alias('model.crate') crate;
  @alias('model.requestedVersion') requestedVersion;
  @alias('model.version') currentVersion;

  get isOwner() {
    return this.crate.owner_user.findBy('id', this.session.currentUser?.id);
  }

  @alias('loadReadmeTask.last.value') readme;

  @task *loadReadmeTask() {
    let version = this.currentVersion;

    let readme = version.loadReadmeTask.lastSuccessful
      ? version.loadReadmeTask.lastSuccessful.value
      : yield version.loadReadmeTask.perform();

    if (typeof document !== 'undefined') {
      setTimeout(() => {
        let e = document.createEvent('CustomEvent');
        e.initCustomEvent('hashchange', true, true);
        window.dispatchEvent(e);
      });
    }

    return readme;
  }
}
