import Ember from 'ember';
import Crate from 'cargo/models/crate';

export default Ember.Route.extend({
    afterModel: function(data) {
      console.log("afterModel");
        if (data instanceof Crate) {
            return data.get('reverse_dependencies');
        } else {
            return data.crate.get('reverse_dependencies');
        }
    },

    setupController: function(controller, data) {
        if (data instanceof Crate) {
            data = {crate: data, reverse_dependencies: null};
        }
        this._super(controller, data.crate);
    },
});
