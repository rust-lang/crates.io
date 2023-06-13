import Model, { attr, hasMany } from '@ember-data/model';
import { waitForPromise } from '@ember/test-waiters';

import { apiAction } from '@mainmatter/ember-api-actions';

export default class Crate extends Model {
  @attr name;
  @attr downloads;
  @attr recent_downloads;
  @attr('date') created_at;
  @attr('date') updated_at;
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

  /**
   * This is the default version that will be shown when visiting the crate
   * details page. Note that this can be `undefined` if all versions of the crate
   * have been yanked.
   * @return {string}
   */
  get defaultVersion() {
    if (this.max_stable_version) {
      return this.max_stable_version;
    }
    if (this.max_version && this.max_version !== '0.0.0') {
      return this.max_version;
    }
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
