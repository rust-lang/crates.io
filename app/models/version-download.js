import Model, { belongsTo, attr } from '@ember-data/model';

export default class VersionDownload extends Model {
  @attr('number') downloads;
  @attr('date') date;

  @belongsTo('version', { async: false }) version;
}
