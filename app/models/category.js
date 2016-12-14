import DS from 'ember-data';

export default DS.Model.extend({
    category: DS.attr('string'),
    slug: DS.attr('string'),
    description: DS.attr('string'),
    created_at: DS.attr('date'),
    crates_cnt: DS.attr('number'),

    subcategories: DS.attr(),

    crates: DS.hasMany('crate', { async: true })
});
