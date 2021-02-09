import Model, { attr, belongsTo, hasMany } from '@ember-data/model';
import { computed } from '@ember/object';
import { alias } from '@ember/object/computed';

import { task } from 'ember-concurrency';
import semverParse from 'semver/functions/parse';

import ajax from '../utils/ajax';

const EIGHT_DAYS = 8 * 24 * 60 * 60 * 1000;

export default class Version extends Model {
  @attr num;
  @attr dl_path;
  @attr readme_path;
  @attr('date') created_at;
  @attr('date') updated_at;
  @attr downloads;
  @attr features;
  @attr yanked;
  @attr license;
  @attr crate_size;

  @belongsTo('crate', { async: false }) crate;

  @belongsTo('user', { async: false }) published_by;
  @hasMany('users', { async: true }) authors;
  @hasMany('dependency', { async: true }) dependencies;
  @hasMany('version-download', { async: true }) version_downloads;

  @computed('crate', function () {
    return this.belongsTo('crate').id();
  })
  crateName;

  get isNew() {
    return Date.now() - this.created_at.getTime() < EIGHT_DAYS;
  }

  get semver() {
    return semverParse(this.num);
  }

  get isPrerelease() {
    return this.semver.prerelease.length !== 0;
  }

  get releaseTrack() {
    let { semver } = this;
    return `${semver.major}.${semver.major === 0 ? semver.minor : 'x'}`;
  }

  get isHighestOfReleaseTrack() {
    if (this.isPrerelease) {
      return false;
    }

    let { crate, semver, releaseTrack } = this;
    let { versions } = crate;
    // find all other non-prerelease versions on the same release track
    let sameTrackVersions = versions.filter(it => it !== this && !it.isPrerelease && it.releaseTrack === releaseTrack);
    // check if we're the "highest"
    return sameTrackVersions.every(it => it.semver.compare(semver) === -1);
  }

  get featureList() {
    let { features } = this;
    if (typeof features !== 'object' || features === null) {
      return [];
    }

    let defaultFeatures = features.default ?? [];
    return Object.keys(features)
      .filter(name => name !== 'default')
      .sort()
      .map(name => ({ name, isDefault: defaultFeatures.includes(name), dependencies: features[name] }));
  }

  @alias('loadAuthorsTask.last.value') authorNames;

  @(task(function* () {
    // trigger the async relationship to load the content
    let authors = yield this.authors;
    return authors.meta.names;
  }).keepLatest())
  loadAuthorsTask;

  @alias('loadDepsTask.last.value.normal') normalDependencies;
  @alias('loadDepsTask.last.value.build') buildDependencies;
  @alias('loadDepsTask.last.value.dev') devDependencies;

  @(task(function* () {
    // trigger the async relationship to load the content
    let dependencies = yield this.dependencies;

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

  @task(function* () {
    return yield ajax(`https://docs.rs/crate/${this.crateName}/${this.num}/builds.json`);
  })
  loadDocsBuildsTask;

  @computed('loadDocsBuildsTask.lastSuccessful.value')
  get hasDocsRsLink() {
    let docsBuilds = this.loadDocsBuildsTask.lastSuccessful?.value;
    return docsBuilds && docsBuilds.length !== 0 && docsBuilds[0].build_status === true;
  }

  get docsRsLink() {
    if (this.hasDocsRsLink) {
      return `https://docs.rs/${this.crateName}/${this.num}`;
    }
  }

  get documentationLink() {
    let crateDocsLink = this.crate.documentation;

    // if this is *not* a docs.rs link we'll return it directly
    if (crateDocsLink && !crateDocsLink.startsWith('https://docs.rs/')) {
      return crateDocsLink;
    }

    // if we know about a successful docs.rs build, we'll return a link to that
    let { docsRsLink } = this;
    if (docsRsLink) {
      return docsRsLink;
    }

    // finally, we'll return the specified documentation link, whatever it is
    if (crateDocsLink) {
      return crateDocsLink;
    }

    return null;
  }

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
