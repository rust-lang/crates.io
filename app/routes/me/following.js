import Ember from 'ember';
import AuthenticatedRoute from 'cargo/mixins/authenticated-route';

export default Ember.Route.extend(AuthenticatedRoute, {
    queryParams: {
        page: { refreshModel: true },
        sort: { refreshModel: true },
    },

    model(params) {
        params.following = 1;
        return this.store.query('crate', params);
    },
});

