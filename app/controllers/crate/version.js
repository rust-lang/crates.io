import Controller from '@ember/controller';
import { action } from '@ember/object';
import { inject as service } from '@ember/service';
import { tracked } from '@glimmer/tracking';

import { task } from 'ember-concurrency';
import { alias } from 'macro-decorators';

export default class CrateVersionController extends Controller {
  @service session;

  get downloadsContext() {
    return this.requestedVersion ? this.currentVersion : this.crate;
  }

  @tracked stackedGraph = true;

  @action setStackedGraph() {
    this.stackedGraph = true;
  }

  @action setUnstackedGraph() {
    this.stackedGraph = false;
  }

  @alias('downloadsContext.version_downloads.content') downloads;
  @alias('model.crate') crate;
  @alias('model.requestedVersion') requestedVersion;
  @alias('model.version') currentVersion;

  get isOwner() {
    return this.crate.owner_user.findBy('id', this.session.currentUser?.id);
  }

  @alias('loadReadmeTask.last.value') readme;

  loadReadmeTask = task(async () => {
    let version = this.currentVersion;

    let readme = version.loadReadmeTask.lastSuccessful
      ? version.loadReadmeTask.lastSuccessful.value
      : await version.loadReadmeTask.perform();

    if (typeof document !== 'undefined') {
      setTimeout(() => {
        let e = document.createEvent('CustomEvent');
        e.initCustomEvent('hashchange', true, true);
        window.dispatchEvent(e);
      });
    }

    return readme;
  });
}
