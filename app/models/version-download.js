import Model, { belongsTo, attr } from '@ember-data/model';

export default class VersionDownload extends Model {
  @attr downloads;
  @attr date;

  @belongsTo('version', { async: false }) version;
}
