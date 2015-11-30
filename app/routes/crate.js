import Ember from 'ember';

export default Ember.Route.extend({
    model(params) {
        return this.store.find('crate', params.crate_id).catch(e => {
            if (e.errors.any(e => e.detail === 'Not Found')) {
                this.controllerFor('application').set('nextFlashError', `Crate '${params.crate_id}' does not exist`);
                return this.replaceWith('index');
            }
        });
    },
});
