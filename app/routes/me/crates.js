import Ember from 'ember';
import AuthenticatedRoute from 'cargo/mixins/authenticated-route';

export default Ember.Route.extend(AuthenticatedRoute, {
    queryParams: {
        page: { refreshModel: true },
        sort: { refreshModel: true },
    },

    model(params) {
        params.user_id = this.session.get('currentUser.id');
        return this.store.find('crate', params);
    },
});

