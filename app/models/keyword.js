import Model, { attr, hasMany } from '@ember-data/model';

export default class Keyword extends Model {
  @attr keyword;
  @attr('date') created_at;
  @attr crates_cnt;

  @hasMany('crate', { async: true }) crates;
}
