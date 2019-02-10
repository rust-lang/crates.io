import { computed } from '@ember/object';
import DS from 'ember-data';

export default DS.Model.extend({
    num: DS.attr('string'),
    dl_path: DS.attr('string'),
    readme_path: DS.attr('string'),
    created_at: DS.attr('date'),
    updated_at: DS.attr('date'),
    downloads: DS.attr('number'),
    yanked: DS.attr('boolean'),
    license: DS.attr('string'),

    crate: DS.belongsTo('crate', {
        async: false,
    }),
    authors: DS.hasMany('users', { async: true }),
    dependencies: DS.hasMany('dependency', { async: true }),
    version_downloads: DS.hasMany('version-download', { async: true }),

    crateName: computed('crate', function() {
        return this.belongsTo('crate').id();
    }),
    crate_size: DS.attr('number'),
});
