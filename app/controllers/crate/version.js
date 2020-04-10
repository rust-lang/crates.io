import ArrayProxy from '@ember/array/proxy';
import Controller from '@ember/controller';
import { computed } from '@ember/object';
import { alias, readOnly, gt } from '@ember/object/computed';
import PromiseProxyMixin from '@ember/object/promise-proxy-mixin';
import { inject as service } from '@ember/service';

import { task } from 'ember-concurrency';
import moment from 'moment';

import ajax from '../../utils/ajax';

const NUM_VERSIONS = 5;

const PromiseArray = ArrayProxy.extend(PromiseProxyMixin);

export default Controller.extend({
  session: service(),

  isDownloading: false,

  downloadsContext: computed('requestedVersion', 'currentVersion', 'crate', function () {
    return this.requestedVersion ? this.currentVersion : this.crate;
  }),
  downloads: alias('downloadsContext.version_downloads'),
  extraDownloads: alias('downloads.content.meta.extra_downloads'),

  crate: alias('model.crate'),
  requestedVersion: alias('model.requestedVersion'),
  currentVersion: alias('model.version'),
  crateTomlText: computed('crate.name', 'currentVersion.num', function () {
    return `${this.get('crate.name')} = "${this.get('currentVersion.num')}"`;
  }),
  keywords: alias('crate.keywords'),
  categories: alias('crate.categories'),
  badges: alias('crate.badges'),
  isOwner: computed('crate.owner_user', 'session.currentUser.id', function () {
    return this.get('crate.owner_user').findBy('id', this.get('session.currentUser.id'));
  }),
  notYankedOrIsOwner: computed('currentVersion', 'crate.owner_user', 'session.currentUser.id', function () {
    return !this.currentVersion.yanked || this.get('crate.owner_user').findBy('id', this.get('session.currentUser.id'));
  }),

  sortedVersions: readOnly('crate.versions'),

  smallSortedVersions: computed('sortedVersions', function () {
    return this.sortedVersions.slice(0, NUM_VERSIONS);
  }),

  hasMoreVersions: gt('sortedVersions.length', NUM_VERSIONS),

  displayedAuthors: computed('currentVersion.authors.[]', function () {
    return PromiseArray.create({
      promise: this.get('currentVersion.authors').then(authors => {
        let ret = authors.slice();
        let others = authors.get('meta');
        for (let i = 0; i < others.names.length; i++) {
          ret.push({ name: others.names[i] });
        }
        return ret;
      }),
    });
  }),

  anyKeywords: gt('keywords.length', 0),
  anyCategories: gt('categories.length', 0),

  currentDependencies: alias('currentVersion.loadDepsTask.last.value.normal'),
  currentBuildDependencies: alias('currentVersion.loadDepsTask.last.value.build'),
  currentDevDependencies: alias('currentVersion.loadDepsTask.last.value.dev'),

  downloadData: computed('downloads', 'extraDownloads', 'requestedVersion', function () {
    let downloads = this.downloads;
    if (!downloads) {
      return;
    }

    let extra = this.extraDownloads || [];

    let dates = {};
    let versions = [];
    for (let i = 0; i < 90; i++) {
      let now = moment().subtract(i, 'days');
      dates[now.format('MMM D')] = { date: now, cnt: {} };
    }

    downloads.forEach(d => {
      let version_id = d.get('version.id');
      let key = moment(d.get('date')).utc().format('MMM D');
      if (dates[key]) {
        let prev = dates[key].cnt[version_id] || 0;
        dates[key].cnt[version_id] = prev + d.get('downloads');
      }
    });

    extra.forEach(d => {
      let key = moment(d.date).utc().format('MMM D');
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
    if (extra.length > 0) {
      versions.push({
        id: null,
        num: 'Other',
      });
    }

    let headers = ['Date'];
    versions.sort(b => b.num).reverse();
    for (let i = 0; i < versions.length; i++) {
      headers.push(versions[i].num);
    }
    let data = [headers];
    for (let date in dates) {
      let row = [dates[date].date.toDate()];
      for (let i = 0; i < versions.length; i++) {
        row.push(dates[date].cnt[versions[i].id] || 0);
      }
      data.push(row);
    }

    return data;
  }),

  loadReadmeTask: task(function* () {
    if (this.currentVersion.get('readme_path')) {
      try {
        let r = yield fetch(this.currentVersion.get('readme_path'));
        if (r.ok) {
          this.crate.set('readme', yield r.text());

          if (typeof document !== 'undefined') {
            setTimeout(() => {
              let e = document.createEvent('CustomEvent');
              e.initCustomEvent('hashchange', true, true);
              window.dispatchEvent(e);
            });
          }
        } else {
          this.crate.set('readme', null);
        }
      } catch (error) {
        this.crate.set('readme', null);
      }
    }
  }),

  documentationLink: computed(
    'crate.{documentation,name}',
    'currentVersion.num',
    'loadDocsBuilds.lastSuccessful.value',
    function () {
      // if this is *not* a docs.rs link we'll return it directly
      if (this.crate.documentation && !this.crate.documentation.startsWith('https://docs.rs/')) {
        return this.crate.documentation;
      }

      // if we know about a successful docs.rs build, we'll return a link to that
      if (this.loadDocsBuilds.lastSuccessful) {
        let docsBuilds = this.loadDocsBuilds.lastSuccessful.value;
        if (docsBuilds.length > 0 && docsBuilds[0].build_status === true) {
          return `https://docs.rs/${this.crate.name}/${this.currentVersion.num}`;
        }
      }

      // finally, we'll return the specified documentation link, whatever it is
      if (this.crate.documentation) {
        return this.crate.documentation;
      }

      return null;
    },
  ),

  loadDocsBuilds: task(function* () {
    return yield ajax(`https://docs.rs/crate/${this.crate.name}/${this.currentVersion.num}/builds.json`);
  }),
});
