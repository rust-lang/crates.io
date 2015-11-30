import Ember from 'ember';

export default Ember.Route.extend({
    queryParams: {
        page: { refreshModel: true },
        sort: { refreshModel: true },
    },

    model(params) {
        params.keyword = this.modelFor('keyword').id;
        return this.store.query('crate', params);
    },

    setupController(controller, model) {
        controller.set('keyword', this.modelFor('keyword'));
        this._super(controller, model);
    },
});
