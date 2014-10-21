import Ember from 'ember';

export default Ember.Route.extend({
    queryParams: {
        page: { refreshModel: true },
        sort: { refreshModel: true },
    },
    loadedCrates: [],

    afterModel: function(keyword, transition) {
        var params = transition.queryParams;
        params.keyword = keyword.get('keyword');
        var self = this;
        return this.store.find('crate', params).then(function(array) {
            if (self.controllerFor('keyword/index')) {
                self.controllerFor('keyword/index').set('model', array);
            }
            self.set('loadedCrates', array);
        });
    },

    setupController: function(controller, keyword) {
        this._super(controller, this.get('loadedCrates'));
        controller.set('keyword', keyword);
    },
});
