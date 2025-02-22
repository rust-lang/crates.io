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
  @attr num_versions;
  @attr yanked;
  @attr max_version;
  @attr max_stable_version;
  @attr newest_version;

  @attr description;
  @attr homepage;
  @attr documentation;
  @attr repository;

  /**
   * This isn't an attribute in the crate response.
   * It's actually the `meta` attribute that belongs to `versions`
   * and needs to be assigned to `crate` manually.
   * @type {Object.<string, {highest: string}>?}
   **/
  @attr release_tracks;

  @hasMany('version', { async: true, inverse: 'crate' }) versions;
  @hasMany('team', { async: true, inverse: null }) owner_team;
  @hasMany('user', { async: true, inverse: null }) owner_user;
  @hasMany('version-download', { async: true, inverse: null }) version_downloads;
  @hasMany('keyword', { async: true, inverse: null }) keywords;
  @hasMany('category', { async: true, inverse: null }) categories;
  @hasMany('dependency', { async: true, inverse: null }) reverse_dependencies;

  /** @return {Map<number, import("../models/version").default>} */
  @cached
  get loadedVersionsById() {
    let versionsRef = this.hasMany('versions');
    let values = versionsRef.value();
    return new Map(values?.map(ref => [ref.id, ref]));
  }

  /** @return {Map<string, import("../models/version").default>} */
  @cached
  get loadedVersionsByNum() {
    let versionsRef = this.hasMany('versions');
    let values = versionsRef.value();
    return new Map(values?.map(ref => [ref.num, ref]));
  }

  @cached get releaseTrackSet() {
    let { release_tracks } = this;
    let nums = release_tracks ? Object.values(this.release_tracks).map(it => it.highest) : [];
    return new Set(nums);
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
