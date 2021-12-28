import Model, { attr, hasMany } from '@ember-data/model';

import { memberAction } from 'ember-api-actions';

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

  follow = memberAction({ type: 'PUT', path: 'follow' });
  unfollow = memberAction({ type: 'DELETE', path: 'follow' });

  inviteOwner = memberAction({
    type: 'PUT',
    path: 'owners',
    before(username) {
      return { owners: [username] };
    },
    after(response) {
      if (response.ok) {
        return response;
      } else {
        throw response;
      }
    },
  });

  removeOwner = memberAction({
    type: 'DELETE',
    path: 'owners',
    before(username) {
      return { owners: [username] };
    },
    after(response) {
      if (response.ok) {
        return response;
      } else {
        throw response;
      }
    },
  });
}
