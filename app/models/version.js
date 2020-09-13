import Model, { attr, belongsTo, hasMany } from '@ember-data/model';
import { computed } from '@ember/object';
import { alias } from '@ember/object/computed';

import { task } from 'ember-concurrency';

export default class Version extends Model {
  @attr('string') num;
  @attr('string') description;
  @attr('string') dl_path;
  @attr('string') readme_path;
  @attr('date') created_at;
  @attr('date') updated_at;
  @attr('number') downloads;
  @attr('boolean') yanked;
  @attr('string') license;
  @attr('number') crate_size;

  @belongsTo('crate', { async: false }) crate;

  @hasMany('users', { async: true }) authors;
  @hasMany('dependency', { async: true }) dependencies;
  @hasMany('version-download', { async: true }) version_downloads;

  @computed('crate', function () {
    return this.belongsTo('crate').id();
  })
  crateName;

  @alias('loadAuthorsTask.last.value') authorNames;

  @(task(function* () {
    let authors = yield this.get('authors');
    return authors.meta.names;
  }).keepLatest())
  loadAuthorsTask;

  @alias('loadDepsTask.last.value.normal') normalDependencies;
  @alias('loadDepsTask.last.value.build') buildDependencies;
  @alias('loadDepsTask.last.value.dev') devDependencies;

  @(task(function* () {
    let dependencies = yield this.get('dependencies');

    let normal = dependencies.filterBy('kind', 'normal').uniqBy('crate_id');
    let build = dependencies.filterBy('kind', 'build').uniqBy('crate_id');
    let dev = dependencies.filterBy('kind', 'dev').uniqBy('crate_id');

    return { normal, build, dev };
  }).keepLatest())
  loadDepsTask;

  @(task(function* () {
    if (this.readme_path) {
      let response = yield fetch(this.readme_path);
      if (!response.ok) {
        throw new Error(`README request for ${this.crateName} v${this.num} failed`);
      }

      return yield response.text();
    }
  }).keepLatest())
  loadReadmeTask;

  @(task(function* () {
    let response = yield fetch(`/api/v1/crates/${this.crate.id}/${this.num}/yank`, { method: 'DELETE' });
    if (!response.ok) {
      throw new Error(`Yank request for ${this.crateName} v${this.num} failed`);
    }
    this.set('yanked', true);

    return yield response.text();
  }).keepLatest())
  yankTask;

  @(task(function* () {
    let response = yield fetch(`/api/v1/crates/${this.crate.id}/${this.num}/unyank`, { method: 'PUT' });
    if (!response.ok) {
      throw new Error(`Unyank request for ${this.crateName} v${this.num} failed`);
    }
    this.set('yanked', false);

    return yield response.text();
  }).keepLatest())
  unyankTask;
}
