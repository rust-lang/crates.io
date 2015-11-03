import Ember from 'ember';
import Crate from 'cargo/models/crate';

export default Ember.Route.extend({
		afterModel: function(data) {
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
            // Redirect to the crate's main page if no documentation
            // URL is found.
            this.transitionTo('crate', crate);
        }
		}
});
