import DS from 'ember-data';

export default DS.Model.extend({
    invited_user_id: DS.attr('number'),
    invited_by_user_id: DS.attr('number'),
    crate_id: DS.attr('number'),
    created_at: DS.attr('date'),
});
