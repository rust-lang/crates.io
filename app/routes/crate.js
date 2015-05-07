import Ember from 'ember';
import Version from 'cargo/models/version';

export default Ember.Route.extend({
    model(params) {
        var parts = params.crate_id.split('/');
        var crate_id = parts[0];
        var version = null;
        if (parts.length > 1 && parts[1].length > 0) {
            version = parts[1];
        }

        var crate = this.store.find('crate', crate_id).catch((e) => {
          if (e.status === 404) {
            this.controllerFor('application').set('nextFlashError', 'No crate named: ' + params.crate_id);
            return this.transitionTo('index');
          }
        });

        return Ember.RSVP.hash({
          crate,
          version
        });
    },

    serialize(model) {
        if (model instanceof Version) {
            var crate = model.get('crate').get('name');

            return {
              crate_id: crate + '/' + model.get('num')
            };
        } else {
            return {
              crate_id: model.get('id')
            };
        }
    },
});
