import Model, { attr, belongsTo, hasMany } from '@ember-data/model';
import { computed } from '@ember/object';

export default Model.extend({
  num: attr('string'),
  dl_path: attr('string'),
  readme_path: attr('string'),
  created_at: attr('date'),
  updated_at: attr('date'),
  downloads: attr('number'),
  yanked: attr('boolean'),
  license: attr('string'),

  crate: belongsTo('crate', {
    async: false,
  }),
  authors: hasMany('users', { async: true }),
  dependencies: hasMany('dependency', { async: true }),
  version_downloads: hasMany('version-download', { async: true }),

  crateName: computed('crate', function() {
    return this.belongsTo('crate').id();
  }),
  crate_size: attr('number'),
});
