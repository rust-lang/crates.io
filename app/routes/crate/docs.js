import Ember from 'ember';
import Crate from '../../models/crate';

export default Ember.Route.extend({
    setupController: function(controller, data) {
        var crate;

        if (data instanceof Crate) {
            crate = data;
        } else {
            crate = data.crate;
        }

        var documentation = crate.get('documentation');

        if (documentation) {
            window.location = documentation;
        } else {
            // Redirect to the crate's main page and show a flash error if
            // no documentation is found
            var message = 'Crate does not supply a documentation URL';
            this.controllerFor('application').set('nextFlashError', message);
            this.replaceWith('crate', crate);
        }

        this._super(controller, crate);
    },
});
