import Model, { attr, hasMany } from '@ember-data/model';
import { map, sort } from '@ember/object/computed';

import { memberAction } from 'ember-api-actions';

export default class Crate extends Model {
  @attr('string') name;
  @attr('number') downloads;
  @attr('number') recent_downloads;
  @attr('date') created_at;
  @attr('date') updated_at;
  @attr('string') max_version;
  @attr('string') newest_version;

  @attr('string') description;
  @attr('string') homepage;
  @attr('string') wiki;
  @attr('string') mailing_list;
  @attr('string') issues;
  @attr('string') documentation;
  @attr('string') repository;
  @attr('boolean') exact_match;

  @hasMany('versions', { async: true }) versions;
  @attr() badges;
  @map('badges', badge => ({
    ...badge,
    component_name: `badge-${badge.badge_type}`,
  }))
  enhanced_badges;

  badge_sort = ['badge_type'];
  @sort('enhanced_badges', 'badge_sort') annotated_badges;

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
  });
}
