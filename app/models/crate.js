import Model, { attr, hasMany } from '@ember-data/model';

import { memberAction } from 'ember-api-actions';

export default class Crate extends Model {
  @attr name;
  @attr downloads;
  @attr recent_downloads;
  @attr('date') created_at;
  @attr('date') updated_at;
  @attr max_version;
  @attr newest_version;

  @attr description;
  @attr homepage;
  @attr wiki;
  @attr mailing_list;
  @attr issues;
  @attr documentation;
  @attr repository;
  @attr exact_match;

  @hasMany('versions', { async: true }) versions;

  @hasMany('users', { async: true }) owners;
  @hasMany('teams', { async: true }) owner_team;
  @hasMany('users', { async: true }) owner_user;
  @hasMany('version-download', { async: true }) version_downloads;
  @hasMany('keywords', { async: true }) keywords;
  @hasMany('categories', { async: true }) categories;
  @hasMany('dependency', { async: true }) reverse_dependencies;

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
