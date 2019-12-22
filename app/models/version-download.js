import Model, { belongsTo, attr } from '@ember-data/model';

export default Model.extend({
  version: belongsTo('version', {
    async: false,
  }),
  downloads: attr('number'),
  date: attr('date'),
});
