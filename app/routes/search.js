import Ember from 'ember';

export default Ember.Route.extend({
    queryParams: {
        q: { refreshModel: true },
        page: { refreshModel: true },
    },

    model(params) {
        return this.store.query('crate', params);
    },
});
