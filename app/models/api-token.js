import DS from 'ember-data';

export default DS.Model.extend({
    name: DS.attr('string'),
    token: DS.attr('string'),
    created_at: DS.attr('date'),
    last_used_at: DS.attr('date'),
});
