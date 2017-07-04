import Ember from 'ember';

export default Ember.Route.extend({
    queryParams: {
        q: { refreshModel: true },
        page: { refreshModel: true },
        sort: { refreshModel: true },
    },

    model(params) {
        if (params.q !== null) {
            params.q = params.q.trim();
        }

        return this.store.query('crate', params);
    },
});
