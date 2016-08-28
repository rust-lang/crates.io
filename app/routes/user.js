import Ember from 'ember';

export default Ember.Route.extend({
    model(params) {
        return this.store.find('user', params.user_id).catch(e => {
            if (e.errors.any(e => e.detail === 'Not Found')) {
                this.controllerFor('application').set('nextFlashError', `User '${params.user_id}' does not exist`);
                return this.replaceWith('index');
            }
        });
    },
});
