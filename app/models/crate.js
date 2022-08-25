import Model, { attr, hasMany } from '@ember-data/model';

import { customAction } from '../utils/custom-action';

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

  @hasMany('versions', { async: true }) versions;

  @hasMany('teams', { async: true }) owner_team;
  @hasMany('users', { async: true }) owner_user;
  @hasMany('version-download', { async: true }) version_downloads;
  @hasMany('keywords', { async: true }) keywords;
  @hasMany('categories', { async: true }) categories;
  @hasMany('dependency', { async: true }) reverse_dependencies;

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
    return await customAction(this, { method: 'PUT', path: 'follow' });
  }

  async unfollow() {
    return await customAction(this, { method: 'DELETE', path: 'follow' });
  }

  async inviteOwner(username) {
    let response = await customAction(this, { method: 'PUT', path: 'owners', data: { owners: [username] } });
    if (response.ok) {
      return response;
    } else {
      throw response;
    }
  }

  async removeOwner(username) {
    let response = await customAction(this, { method: 'DELETE', path: 'owners', data: { owners: [username] } });
    if (response.ok) {
      return response;
    } else {
      throw response;
    }
  }
}
