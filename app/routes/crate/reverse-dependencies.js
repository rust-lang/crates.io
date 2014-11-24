import Ember from 'ember';
import Crate from 'cargo/models/crate';

export default Ember.Route.extend({
    queryParams: {
        page: { refreshModel: true },
    },

    crate: null,
    reverse_dependencies: null,
    params: null,

    model: function(params, transition) {
        this.set('params', params);
        return this._super(params, transition);
    },

    afterModel: function(data) {
        var crate;
        if (data instanceof Crate) {
            crate = data;
        } else {
            crate = data.crate;
        }
        var self = this;

        var params = this.get('params');
        params.reverse = true;
        params.crate = crate;

        return this.store.find('dependency', params).then(function(deps) {
            var controller = self.controllerFor('crate/reverse_dependencies');
            if (controller) {
                controller.set('model', deps);
            }
            self.set('reverse_dependencies', deps);
            self.set('crate', crate);
        });
    },

    setupController: function(controller) {
        this._super(controller, this.get('reverse_dependencies'));
        controller.set('crate', this.get('crate'));
    },
});
