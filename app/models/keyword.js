import Model, { attr, hasMany } from '@ember-data/model';

export default class Keyword extends Model {
  @attr('string') keyword;
  @attr('date') created_at;
  @attr('number') crates_cnt;

  @hasMany('crate', { async: true }) crates;
}
