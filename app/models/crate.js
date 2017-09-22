import { map, sort } from '@ember/object/computed';
import DS from 'ember-data';

export default DS.Model.extend({
    name: DS.attr('string'),
    downloads: DS.attr('number'),
    recent_downloads: DS.attr('number'),
    created_at: DS.attr('date'),
    updated_at: DS.attr('date'),
    max_version: DS.attr('string'),

    description: DS.attr('string'),
    homepage: DS.attr('string'),
    wiki: DS.attr('string'),
    mailing_list: DS.attr('string'),
    issues: DS.attr('string'),
    documentation: DS.attr('string'),
    repository: DS.attr('string'),
    exact_match: DS.attr('boolean'),

    versions: DS.hasMany('versions', { async: true }),
    badges: DS.attr(),
    enhanced_badges: map('badges', badge => ({
        ...badge,
        component_name: `badge-${badge.badge_type}`
    })),
    badge_sort: ['badge_type'],
    annotated_badges: sort('enhanced_badges', 'badge_sort'),
    owners: DS.hasMany('users', { async: true }),
    owner_team: DS.hasMany('teams', { async: true }),
    owner_user: DS.hasMany('users', { async: true }),
    version_downloads: DS.hasMany('version-download', { async: true }),
    keywords: DS.hasMany('keywords', { async: true }),
    categories: DS.hasMany('categories', { async: true }),
    reverse_dependencies: DS.hasMany('dependency', { async: true }),

    follow() {
        return this.store.adapterFor('crate').follow(this.get('id'));
    },

    unfollow() {
        return this.store.adapterFor('crate').unfollow(this.get('id'));
    },
});
