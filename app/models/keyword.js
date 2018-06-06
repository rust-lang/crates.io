import DS from 'ember-data';

export default DS.Model.extend({
    keyword: DS.attr('string'),
    created_at: DS.attr('date'),
    crates_cnt: DS.attr('number'),

    crates: DS.hasMany('crate', { async: true }),
});
