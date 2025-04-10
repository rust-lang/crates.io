import Controller from '@ember/controller';
import { action } from '@ember/object';
import { service } from '@ember/service';

import { dropTask } from 'ember-concurrency';
import { reads } from 'macro-decorators';

import ajax from '../utils/ajax';

export default class IndexController extends Controller {
  @service store;

  @reads('dataTask.lastSuccessful.value') model;

  get hasData() {
    return this.dataTask.lastSuccessful && !this.dataTask.isRunning;
  }

  @action fetchData() {
    return this.dataTask.perform().catch(() => {
      // we ignore errors here because they are handled in the template already
    });
  }

  dataTask = dropTask(async () => {
    let data = await ajax('/api/v1/summary');

    addCrates(this.store, data.new_crates);
    addCrates(this.store, data.most_downloaded);
    addCrates(this.store, data.just_updated);
    addCrates(this.store, data.most_recently_downloaded);

    return data;
  });
}

function addCrates(store, crates) {
  for (let i = 0; i < crates.length; i++) {
    crates[i] = store.push(store.normalize('crate', crates[i]));
  }
}
