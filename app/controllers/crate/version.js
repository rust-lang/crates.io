import Controller from '@ember/controller';
import { action } from '@ember/object';
import { inject as service } from '@ember/service';
import { tracked } from '@glimmer/tracking';

import { task } from 'ember-concurrency';
import { alias } from 'macro-decorators';

export default class CrateVersionController extends Controller {
  @service mermaid;
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

    // If the README contains `language-mermaid` we ensure that the `mermaid` library has loaded before we continue
    if (readme && readme.includes('language-mermaid') && !this.mermaid.loadTask.lastSuccessful?.value) {
      try {
        await this.mermaid.loadTask.perform();
      } catch (error) {
        // If we failed to load the library due to network issues, it is not the end of the world, and we just log
        // the error to the console.
        console.error(error);
      }
    }

    if (typeof document !== 'undefined') {
      setTimeout(() => {
        let e = new CustomEvent('hashchange');
        window.dispatchEvent(e);
      });
    }

    return readme;
  });
}
