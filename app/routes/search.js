import Ember from 'ember';

export default Ember.Route.extend({
    title: Ember.computed.reads("controller.name"),

    queryParams: {
        q: { refreshModel: true },
        page: { refreshModel: true },
    },

    model: function(params) {
        return this.store.find('crate', params);
    },
});
