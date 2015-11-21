import Ember from 'ember';

export default Ember.Route.extend({
    queryParams: {
        page: { refreshModel: true },
    },

    model(params) {
        params.reverse = true;
        params.crate = this.modelFor('crate').crate;

        return this.store.query('dependency', params);
    },

    setupController(controller, model) {
        controller.set('crate', this.modelFor('crate').crate);
        this._super(controller, model);
    },
});
