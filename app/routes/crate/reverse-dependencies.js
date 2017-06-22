import Ember from 'ember';

export default Ember.Route.extend({
    queryParams: {
        page: { refreshModel: true },
    },

    model(params) {
        params.reverse = true;
        params.crate = this.modelFor('crate');

        return this.store.query('dependency', params);
    }
});
