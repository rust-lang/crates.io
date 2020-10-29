import Model, { belongsTo, attr } from '@ember-data/model';

export default class VersionDownload extends Model {
  /** @type number */
  @attr downloads;
  /** @type string */
  @attr date;

  @belongsTo('version', { async: false }) version;
}
