import Model, { attr, belongsTo, hasMany } from '@ember-data/model';
import { service } from '@ember/service';
import { waitForPromise } from '@ember/test-waiters';
import { cached } from '@glimmer/tracking';

import { apiAction } from '@mainmatter/ember-api-actions';
import { keepLatestTask, task } from 'ember-concurrency';
import { alias } from 'macro-decorators';
import semverParse from 'semver/functions/parse';

import ajax from '../utils/ajax';
import { addRegistryUrl } from '../utils/purl';

const EIGHT_DAYS = 8 * 24 * 60 * 60 * 1000;

export default class Version extends Model {
  @service releaseTracks;

  @attr num;
  @attr dl_path;
  @attr readme_path;
  @attr('date') created_at;
  @attr downloads;
  @attr features;
  @attr yanked;
  @attr license;
  @attr crate_size;

  /**
   * The minimum supported Rust version of this crate version.
   * @type {string | null}
   */
  @attr rust_version;

  /**
   * The Rust edition required to compile this crate version.
   * @type {string | null}
   */
  @attr edition;

  /** @type {boolean | null} */
  @attr has_lib;
  /** @type {string[] | null} */
  @attr bin_names;

  /**
   * Information about the trusted publisher that published this version, if any.
   * @type {Object | null}
   */
  @attr trustpub_data;

  /**
   * The name of the trusted publisher that published this version, if any.
   * @type {string | null}
   */
  get trustpubPublisher() {
    return this.trustpub_data?.provider === 'github' ? 'GitHub' : null;
  }

  /**
   * The URL to the trusted publisher that published this version, if any.
   * @type {string | null}
   */
  get trustpubUrl() {
    return this.trustpub_data?.provider === 'github'
      ? `https://github.com/${this.trustpub_data.repository}/actions/runs/${this.trustpub_data.run_id}`
      : null;
  }

  @belongsTo('crate', { async: false, inverse: 'versions' }) crate;

  @belongsTo('user', { async: false, inverse: null }) published_by;
  @hasMany('dependency', { async: true, inverse: 'version' }) dependencies;
  @hasMany('version-download', { async: true, inverse: null }) version_downloads;

  get crateName() {
    return this.belongsTo('crate').id();
  }

  /**
   * Returns the Package URL (PURL) for this version.
   * @type {string}
   */
  get purl() {
    let basePurl = `pkg:cargo/${this.crateName}@${this.num}`;
    return addRegistryUrl(basePurl);
  }

  get editionMsrv() {
    if (this.edition === '2018') {
      return '1.31.0';
    } else if (this.edition === '2021') {
      return '1.56.0';
    } else if (this.edition === '2024') {
      return '1.85.0';
    }
  }

  get msrv() {
    let rustVersion = this.rust_version;
    if (rustVersion) {
      // add `.0` suffix if the `rust-version` field only has two version components
      return /^[^.]+\.[^.]+$/.test(rustVersion) ? `${rustVersion}.0` : rustVersion;
    }
  }

  get isNew() {
    return Date.now() - this.created_at.getTime() < EIGHT_DAYS;
  }

  get semver() {
    return semverParse(this.num, { loose: true });
  }

  get invalidSemver() {
    return this.semver === null;
  }

  get isPrerelease() {
    if (this.invalidSemver) {
      return false;
    }

    return this.semver.prerelease.length !== 0;
  }

  get releaseTrack() {
    if (this.invalidSemver) {
      return null;
    }

    let { semver } = this;
    return semver.major >= 100 ? String(semver.major) : `${semver.major}.${semver.major === 0 ? semver.minor : 'x'}`;
  }

  @cached get isHighestOfReleaseTrack() {
    if (this.isPrerelease || this.invalidSemver) {
      return false;
    }

    return this.crate?.releaseTrackSet.has(this.num);
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

  @alias('loadDepsTask.last.value.normal') normalDependencies;
  @alias('loadDepsTask.last.value.build') buildDependencies;
  @alias('loadDepsTask.last.value.dev') devDependencies;

  loadDepsTask = keepLatestTask(async () => {
    // trigger the async relationship to load the content
    let dependencies = await this.dependencies;

    let normal = dependencies.filter(d => d.kind === 'normal');
    let build = dependencies.filter(d => d.kind === 'build');
    let dev = dependencies.filter(d => d.kind === 'dev');

    return { normal, build, dev };
  });

  loadReadmeTask = keepLatestTask(async () => {
    if (this.readme_path) {
      let response = await waitForPromise(fetch(this.readme_path));
      if (response.status === 404 || response.status === 403) {
        return;
      }

      if (!response.ok) {
        throw new Error(`README request for ${this.crateName} v${this.num} failed`);
      }

      return await waitForPromise(response.text());
    }
  });

  loadDocsStatusTask = task(async () => {
    return await ajax(`https://docs.rs/crate/${this.crateName}/=${this.num}/status.json`);
  });

  get hasDocsRsLink() {
    let docsStatus = this.loadDocsStatusTask.lastSuccessful?.value;
    return docsStatus?.doc_status === true;
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

  yankTask = keepLatestTask(async () => {
    let data = { version: { yanked: true } };
    let payload = await waitForPromise(apiAction(this, { method: 'PATCH', data }));
    this.store.pushPayload(payload);
    await waitForPromise(this.releaseTracks.refreshTask.perform(this.crateName, true));
  });

  unyankTask = keepLatestTask(async () => {
    let data = { version: { yanked: false } };
    let payload = await waitForPromise(apiAction(this, { method: 'PATCH', data }));
    this.store.pushPayload(payload);
    await waitForPromise(this.releaseTracks.refreshTask.perform(this.crateName, false));
  });

  async rebuildDocs() {
    return await waitForPromise(apiAction(this, { method: 'POST', path: 'rebuild_docs' }));
  }
}
