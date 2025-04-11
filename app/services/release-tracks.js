import Service, { service } from '@ember/service';

import { didCancel, dropTask } from 'ember-concurrency';

import { AjaxError } from '../utils/ajax';

export default class PristineParamsService extends Service {
  @service sentry;
  @service store;

  refreshTask = dropTask(async crateName => {
    let query = {
      include: 'release_tracks',
      name: crateName,
      per_page: 1,
      sort: 'semver',
    };

    try {
      let versions = await this.store.query('version', query);
      let meta = versions.meta;
      if (meta.release_tracks) {
        this.updatePayload(crateName, meta.release_tracks);
      }
    } catch (error) {
      if (!didCancel(error) && !(error instanceof AjaxError)) {
        this.sentry.captureException(error);
      }
    }
  });

  updatePayload(crateName, release_tracks) {
    let payload = {
      crate: {
        id: crateName,
        release_tracks,
      },
    };
    this.store.pushPayload(payload);
  }
}
