import Controller from '@ember/controller';
import { computed } from '@ember/object';
import { alias } from '@ember/object/computed';
import { inject as service } from '@ember/service';

import subDays from 'date-fns/subDays';
import { task } from 'ember-concurrency';

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

  @computed('crate.owner_user', 'session.currentUser.id')
  get isOwner() {
    return this.crate.owner_user.findBy('id', this.session.currentUser?.id);
  }

  @computed('downloads', 'extraDownloads')
  get downloadData() {
    let downloads = this.downloads;
    if (!downloads) {
      return;
    }

    let extra = this.extraDownloads || [];

    let dates = {};
    let versions = new Set([]);

    let now = new Date();
    for (let i = 0; i < 90; i++) {
      let date = subDays(now, i);
      dates[date.toISOString().slice(0, 10)] = { date, cnt: {} };
    }

    downloads.forEach(d => {
      let version_num = d.version.num;

      versions.add(version_num);

      let key = d.date;
      if (dates[key]) {
        let prev = dates[key].cnt[version_num] || 0;
        dates[key].cnt[version_num] = prev + d.downloads;
      }
    });

    extra.forEach(d => {
      let key = d.date;
      if (dates[key]) {
        let prev = dates[key].cnt['Other'] || 0;
        dates[key].cnt['Other'] = prev + d.downloads;
      }
    });

    let versionsList = [...versions].sort();
    if (extra.length !== 0) {
      versionsList.unshift('Other');
    }

    let headers = ['Date', ...versionsList];

    let data = [headers];
    for (let date in dates) {
      let row = [dates[date].date];
      for (let version of versionsList) {
        row.push(dates[date].cnt[version] || 0);
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
}
