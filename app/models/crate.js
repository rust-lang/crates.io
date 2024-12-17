import Model, { attr, hasMany } from '@ember-data/model';
import { assert } from '@ember/debug';
import { waitForPromise } from '@ember/test-waiters';
import { cached } from '@glimmer/tracking';

import { apiAction } from '@mainmatter/ember-api-actions';
import { task } from 'ember-concurrency';

export default class Crate extends Model {
  @attr name;
  @attr downloads;
  @attr recent_downloads;
  @attr('date') created_at;
  @attr('date') updated_at;
  /**
   * This is the default version that will be shown when visiting the crate
   * details page. Note that this value can be `null`, which may be unexpected.
   * @type {string | null}
   */
  @attr default_version;
  @attr yanked;
  @attr max_version;
  @attr max_stable_version;
  @attr newest_version;
  /**
   * @typedef {Object} VersionsMeta
   * @property {number} total
   * @property {string | null} next_page
   * @property {Object.<string, ReleaseTrackDetails>} release_tracks
   *
   * @typedef {Object} ReleaseTrackDetails
   * @property {string} highest
   **/
  /**
   * This isn't an attribute in the crate response.
   * It's actually the `meta` attribute that belongs to `versions`
   * and needs to be assigned to `crate` manually.
   * @type {VersionsMeta | null}
   **/
  @attr versions_meta;

  @attr description;
  @attr homepage;
  @attr documentation;
  @attr repository;

  @hasMany('version', { async: true, inverse: 'crate' }) versions;
  @hasMany('team', { async: true, inverse: null }) owner_team;
  @hasMany('user', { async: true, inverse: null }) owner_user;
  @hasMany('version-download', { async: true, inverse: null }) version_downloads;
  @hasMany('keyword', { async: true, inverse: null }) keywords;
  @hasMany('category', { async: true, inverse: null }) categories;
  @hasMany('dependency', { async: true, inverse: null }) reverse_dependencies;

  @cached get versionIdsBySemver() {
    let { last } = this.loadVersionsTask;
    assert('`loadVersionsTask.perform()` must be called before calling `versionIdsBySemver`', last != null);
    let versions = last?.value ?? [];
    return versions
      .slice()
      .sort(compareVersionBySemver)
      .map(v => v.id);
  }

  @cached get versionIdsByDate() {
    let { last } = this.loadVersionsTask;
    assert('`loadVersionsTask.perform()` must be called before calling `versionIdsByDate`', last != null);
    let versions = last?.value ?? [];
    return versions
      .slice()
      .sort(compareVersionByDate)
      .map(v => v.id);
  }

  @cached get firstVersionId() {
    return this.versionIdsByDate.at(-1);
  }

  @cached get versionsObj() {
    let { last } = this.loadVersionsTask;
    assert('`loadVersionsTask.perform()` must be called before calling `versionsObj`', last != null);
    let versions = last?.value ?? [];
    return Object.fromEntries(versions.slice().map(v => [v.id, v]));
  }

  @cached get releaseTrackSet() {
    let map = new Map();
    let { versionsObj: versions, versionIdsBySemver } = this;
    for (let id of versionIdsBySemver) {
      let { releaseTrack, isPrerelease, yanked } = versions[id];
      if (releaseTrack && !isPrerelease && !yanked && !map.has(releaseTrack)) {
        map.set(releaseTrack, id);
      }
    }
    return new Set(map.values());
  }

  hasOwnerUser(userId) {
    let { last } = this.loadOwnerUserTask;
    assert('`loadOwnerUserTask.perform()` must be called before calling `hasOwnerUser()`', last != null);
    return (last?.value ?? []).some(({ id }) => id === userId);
  }

  get owners() {
    let { last } = this.loadOwnersTask;
    assert('`loadOwnersTask.perform()` must be called before accessing `owners`', last != null);
    return last?.value ?? [];
  }

  async follow() {
    return await waitForPromise(apiAction(this, { method: 'PUT', path: 'follow' }));
  }

  async unfollow() {
    return await waitForPromise(apiAction(this, { method: 'DELETE', path: 'follow' }));
  }

  async inviteOwner(username) {
    let response = await waitForPromise(
      apiAction(this, { method: 'PUT', path: 'owners', data: { owners: [username] } }),
    );
    if (response.ok) {
      return response;
    } else {
      throw response;
    }
  }

  async removeOwner(username) {
    let response = await waitForPromise(
      apiAction(this, { method: 'DELETE', path: 'owners', data: { owners: [username] } }),
    );
    if (response.ok) {
      return response;
    } else {
      throw response;
    }
  }

  loadOwnerUserTask = task(async () => {
    return (await this.owner_user) ?? [];
  });

  loadOwnersTask = task(async () => {
    let [teams, users] = await Promise.all([this.owner_team, this.owner_user]);
    return [...(teams ?? []), ...(users ?? [])];
  });

  loadVersionsTask = task(async ({ reload = false } = {}) => {
    let versionsRef = this.hasMany('versions');
    let fut = reload === true ? versionsRef.reload() : versionsRef.load();
    return (await fut) ?? [];
  });
}

function compareVersionBySemver(a, b) {
  let aSemver = a.semver;
  let bSemver = b.semver;

  if (aSemver === bSemver) {
    return b.created_at - a.created_at;
  } else if (aSemver === null) {
    return 1;
  } else if (bSemver === null) {
    return -1;
  } else {
    return bSemver.compare(aSemver);
  }
}

function compareVersionByDate(a, b) {
  let bDate = b.created_at.getTime();
  let aDate = a.created_at.getTime();

  return bDate === aDate ? parseInt(b.id) - parseInt(a.id) : bDate - aDate;
}
