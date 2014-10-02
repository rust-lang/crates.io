import Ember from 'ember';

export default Ember.Route.extend({
    queryParams: {
        letter: { refreshModel: true },
        page: { refreshModel: true },
        sort: { refreshModel: true },
    },

    model: function(params) {
        return this.store.find('crate', params);
    },
});
