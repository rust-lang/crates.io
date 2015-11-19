import Ember from 'ember';

export default Ember.Route.extend({
    queryParams: {
        page: { refreshModel: true },
        sort: { refreshModel: true },
    },
    loadedCrates: [],

    afterModel(keyword, transition) {
        var params = transition.queryParams;
        params.keyword = keyword.get('keyword');
        return this.store.query('crate', params).then((array) => {
            if (this.controllerFor('keyword/index')) {
                this.controllerFor('keyword/index').set('model', array);
            }
            this.set('loadedCrates', array);
        });
    },

    setupController(controller, keyword) {
        this._super(controller, this.get('loadedCrates'));
        controller.set('keyword', keyword);
    }
});
