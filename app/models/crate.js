import DS from 'ember-data';
import Ember from 'ember';

export default DS.Model.extend({
    name: DS.attr('string'),
    downloads: DS.attr('number'),
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
    license: DS.attr('string'),
    max_build_info_nightly: DS.attr('date'),
    max_build_info_beta: DS.attr('date'),
    max_build_info_stable: DS.attr('string'),

    versions: DS.hasMany('versions', { async: true }),
    badges: DS.attr(),
    enhanced_badges: Ember.computed.map('badges', badge => ({
        // jshint ignore:start
        // needed until https://github.com/jshint/jshint/issues/2991 is fixed
        ...badge,
        // jshint ignore:end
        component_name: `badge-${badge.badge_type}`
    })),
    badge_sort: ['badge_type'],
    annotated_badges: Ember.computed.sort('enhanced_badges', 'badge_sort'),
    owners: DS.hasMany('users', { async: true }),
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
