import Ember from 'ember';

export default Ember.Route.extend({
    queryParams: {
        page: { refreshModel: true },
        sort: { refreshModel: true },
    },

    model(params) {
        params.category = this.modelFor('category').id;
        return this.store.query('crate', params);
    },

    setupController(controller, model) {
        controller.set('category', this.modelFor('category'));
        this._super(controller, model);
    },
});
