import DS from 'ember-data';

export default DS.Model.extend({
    slug: DS.attr('string'),
    description: DS.attr('string'),
});
