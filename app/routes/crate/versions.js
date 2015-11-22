import Ember from 'ember';

export default Ember.Route.extend({
    model() {
        return this.modelFor('crate').crate.get('versions');
    },

    setupController(controller, model) {
        controller.set('crate', this.modelFor('crate').crate);
        this._super(controller, model);
    },
});
