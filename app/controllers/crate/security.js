import Controller from '@ember/controller';
import { service } from '@ember/service';
import { tracked } from '@glimmer/tracking';

import { didCancel, dropTask } from 'ember-concurrency';

import { AjaxError } from '../../utils/ajax';

export default class SearchController extends Controller {
  @service releaseTracks;
  @service sentry;

  @tracked crate;
  @tracked data;

  constructor() {
    super(...arguments);
    this.reset();
  }

  loadMoreTask = dropTask(async () => {
    let { crate } = this;
    let url = `https://rustsec.org/packages/${crate.id}.json`;

    try {
      let response = await fetch(url);
      if (response.status === 404) {
        this.data = [];
      } else if (response.ok) {
        this.data = await response.json();
      } else {
        throw new Error(`HTTP error! status: ${response}`);
      }
    } catch (error) {
      // report unexpected errors to Sentry and ignore `ajax()` errors
      if (!didCancel(error) && !(error instanceof AjaxError)) {
        this.sentry.captureException(error);
      }
    }
  });

  reset() {
    this.crate = undefined;
    this.data = undefined;
  }
}
