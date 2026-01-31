import Controller from '@ember/controller';
import { service } from '@ember/service';
import { tracked } from '@glimmer/tracking';

import { didCancel, dropTask } from 'ember-concurrency';
import { alias } from 'macro-decorators';

import { AjaxError } from '../../utils/ajax';

function defaultVersionsContext() {
  return { data: [], next_page: undefined };
}

export default class SearchController extends Controller {
  @service releaseTracks;
  @service sentry;
  @service session;
  @service store;

  queryParams = ['per_page', 'sort'];
  @tracked sort;
  @tracked per_page = 100;

  @tracked byDate;
  @tracked bySemver;
  /** @type {import("../../models/crate").default} */
  @tracked crate;

  @alias('versionsContext.data') data;
  @alias('versionsContext.next_page') next_page;

  constructor() {
    super(...arguments);
    this.reset();
  }

  get currentSortBy() {
    return this.sort === 'semver' ? 'SemVer' : 'Date';
  }

  get versionsContext() {
    return this.sort === 'semver' ? this.bySemver : this.byDate;
  }

  get sortedVersions() {
    let { loadedVersionsById: versions } = this.crate;
    return this.data.map(id => versions.get(id));
  }

  get isOwner() {
    let userId = this.session.currentUser?.id;
    return this.crate.hasOwnerUser(userId);
  }

  loadMoreTask = dropTask(async () => {
    let { crate, data, next_page, per_page, sort, versionsContext } = this;
    let query;

    if (next_page) {
      let params = new URLSearchParams(next_page);
      params.set('name', crate.name);
      params.delete('include');
      query = Object.fromEntries(params.entries());
    } else {
      if (sort !== 'semver') {
        sort = 'date';
      }
      query = {
        name: crate.name,
        sort,
        per_page,
      };
    }
    if (crate.release_tracks == null) {
      query.include = 'release_tracks';
    }

    try {
      let versions = await this.store.query('version', query);
      let meta = versions.meta;

      let ids = versions.map(it => it.id);
      if (sort === 'semver') {
        this.bySemver = {
          ...versionsContext,
          data: data.concat(ids),
          next_page: meta.next_page,
        };
      } else {
        this.byDate = {
          ...versionsContext,
          data: data.concat(ids),
          next_page: meta.next_page,
        };
      }

      if (meta.release_tracks) {
        this.releaseTracks.updatePayload(crate.id, meta.release_tracks);
      }

      return versions;
    } catch (error) {
      // report unexpected errors to Sentry and ignore `ajax()` errors
      if (!didCancel(error) && !(error instanceof AjaxError)) {
        this.sentry.captureException(error);
      }
    }
  });

  reset() {
    this.crate = undefined;
    this.byDate = defaultVersionsContext();
    this.bySemver = defaultVersionsContext();
  }
}
