import Model, { attr, hasMany } from '@ember-data/model';
import { map, sort } from '@ember/object/computed';

import { memberAction } from 'ember-api-actions';

export default Model.extend({
  name: attr('string'),
  downloads: attr('number'),
  recent_downloads: attr('number'),
  created_at: attr('date'),
  updated_at: attr('date'),
  max_version: attr('string'),
  newest_version: attr('string'),

  description: attr('string'),
  homepage: attr('string'),
  wiki: attr('string'),
  mailing_list: attr('string'),
  issues: attr('string'),
  documentation: attr('string'),
  repository: attr('string'),
  exact_match: attr('boolean'),

  versions: hasMany('versions', { async: true }),
  badges: attr(),
  enhanced_badges: map('badges', badge => ({
    ...badge,
    component_name: `badge-${badge.badge_type}`,
  })),

  // eslint-disable-next-line ember/avoid-leaking-state-in-ember-objects
  badge_sort: ['badge_type'],
  annotated_badges: sort('enhanced_badges', 'badge_sort'),

  owners: hasMany('users', { async: true }),
  owner_team: hasMany('teams', { async: true }),
  owner_user: hasMany('users', { async: true }),
  version_downloads: hasMany('version-download', { async: true }),
  keywords: hasMany('keywords', { async: true }),
  categories: hasMany('categories', { async: true }),
  reverse_dependencies: hasMany('dependency', { async: true }),

  follow: memberAction({ type: 'PUT', path: 'follow' }),
  unfollow: memberAction({ type: 'DELETE', path: 'follow' }),

  inviteOwner: memberAction({
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
  }),

  removeOwner: memberAction({
    type: 'DELETE',
    path: 'owners',
    before(username) {
      return { owners: [username] };
    },
  }),
});
