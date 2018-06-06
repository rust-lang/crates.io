import DS from 'ember-data';

export default DS.Model.extend({
    invited_by_username: DS.attr('string'),
    crate_name: DS.attr('string'),
    crate_id: DS.attr('number'),
    created_at: DS.attr('date'),
    accepted: DS.attr('boolean', { defaultValue: false }),
});
