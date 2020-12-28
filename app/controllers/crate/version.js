import Controller from '@ember/controller';
import { computed } from '@ember/object';
import { alias, gt, readOnly } from '@ember/object/computed';
import { inject as service } from '@ember/service';

import { task } from 'ember-concurrency';
import moment from 'moment';

import ajax from '../../utils/ajax';

const NUM_VERSIONS = 5;

export default class CrateVersionController extends Controller {
  @service session;

  @computed('requestedVersion', 'currentVersion', 'crate')
  get downloadsContext() {
    return this.requestedVersion ? this.currentVersion : this.crate;
  }

  @alias('downloadsContext.version_downloads') downloads;
  @alias('downloads.content.meta.extra_downloads') extraDownloads;
  @alias('model.crate') crate;
  @alias('model.requestedVersion') requestedVersion;
  @alias('model.version') currentVersion;
  @alias('crate.keywords') keywords;
  @alias('crate.categories') categories;

  @computed('crate.owner_user', 'session.currentUser.id')
  get isOwner() {
    return this.crate.owner_user.findBy('id', this.session.currentUser?.id);
  }

  @readOnly('crate.versions') sortedVersions;

  @computed('sortedVersions')
  get smallSortedVersions() {
    return this.sortedVersions.slice(0, NUM_VERSIONS);
  }

  @gt('sortedVersions.length', NUM_VERSIONS) hasMoreVersions;
  @gt('keywords.length', 0) anyKeywords;
  @gt('categories.length', 0) anyCategories;

  @computed('downloads', 'extraDownloads', 'requestedVersion')
  get downloadData() {
    let downloads = this.downloads;
    if (!downloads) {
      return;
    }

    let extra = this.extraDownloads || [];

    let dates = {};
    let versions = [];
    for (let i = 0; i < 90; i++) {
      let now = moment().subtract(i, 'days');
      dates[now.toISOString().slice(0, 10)] = { date: now, cnt: {} };
    }

    downloads.forEach(d => {
      let version_id = d.version.id;
      let key = d.date;
      if (dates[key]) {
        let prev = dates[key].cnt[version_id] || 0;
        dates[key].cnt[version_id] = prev + d.downloads;
      }
    });

    extra.forEach(d => {
      let key = d.date;
      if (dates[key]) {
        let prev = dates[key].cnt[null] || 0;
        dates[key].cnt[null] = prev + d.downloads;
      }
    });
    if (this.requestedVersion) {
      versions.push(this.currentVersion.getProperties('id', 'num'));
    } else {
      this.smallSortedVersions.forEach(version => {
        versions.push(version.getProperties('id', 'num'));
      });
    }
    if (extra.length !== 0) {
      versions.push({
        id: null,
        num: 'Other',
      });
    }

    let headers = ['Date'];
    versions.sort(b => b.num).reverse();
    for (let version of versions) {
      headers.push(version.num);
    }
    let data = [headers];
    for (let date in dates) {
      let row = [dates[date].date.toDate()];
      for (let version of versions) {
        row.push(dates[date].cnt[version.id] || 0);
      }
      data.push(row);
    }

    return data;
  }

  @alias('loadReadmeTask.last.value') readme;

  @task(function* () {
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
  })
  loadReadmeTask;

  @computed('crate.{documentation,name}', 'currentVersion.num', 'loadDocsBuildsTask.lastSuccessful.value')
  get documentationLink() {
    // if this is *not* a docs.rs link we'll return it directly
    if (this.crate.documentation && !this.crate.documentation.startsWith('https://docs.rs/')) {
      return this.crate.documentation;
    }

    // if we know about a successful docs.rs build, we'll return a link to that
    if (this.loadDocsBuildsTask.lastSuccessful) {
      let docsBuilds = this.loadDocsBuildsTask.lastSuccessful.value;
      if (docsBuilds.length !== 0 && docsBuilds[0].build_status === true) {
        return `https://docs.rs/${this.crate.name}/${this.currentVersion.num}`;
      }
    }

    // finally, we'll return the specified documentation link, whatever it is
    if (this.crate.documentation) {
      return this.crate.documentation;
    }

    return null;
  }

  @task(function* () {
    return yield ajax(`https://docs.rs/crate/${this.crate.name}/${this.currentVersion.num}/builds.json`);
  })
  loadDocsBuildsTask;
}
