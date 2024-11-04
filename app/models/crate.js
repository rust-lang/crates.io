import Model, { attr, hasMany } from '@ember-data/model';
import { waitForPromise } from '@ember/test-waiters';

import { apiAction } from '@mainmatter/ember-api-actions';
import { cached } from 'tracked-toolbox';

export default class Crate extends Model {
  @attr name;
  @attr downloads;
  @attr recent_downloads;
  @attr('date') created_at;
  @attr('date') updated_at;
  /**
   * This is the default version that will be shown when visiting the crate
   * details page. Note that this value can be `null`, which may be unexpected.
   * @return {string}
   */
  @attr default_version;
  @attr yanked;
  @attr max_version;
  @attr max_stable_version;
  @attr newest_version;

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
    let versions = this.versions.toArray() ?? [];
    return versions.sort(compareVersionBySemver).map(v => v.id);
  }

  @cached get versionIdsByDate() {
    let versions = this.versions.toArray() ?? [];
    return versions.sort(compareVersionByDate).map(v => v.id);
  }

  @cached get firstVersionId() {
    return this.versionIdsByDate.at(-1);
  }

  @cached get versionsObj() {
    let versions = this.versions.toArray() ?? [];
    return Object.fromEntries(versions.map(v => [v.id, v]));
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

  get owners() {
    let teams = this.owner_team.toArray() ?? [];
    let users = this.owner_user.toArray() ?? [];
    return [...teams, ...users];
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
