import Ember from 'ember';

export default Ember.Route.extend({
    model(params) {
        return this.store.find('category', params.category_id).catch(e => {
            if (e.errors.any(e => e.detail === 'Not Found')) {
                this.controllerFor('application').set('flashError', `Category '${params.category_id}' does not exist`);
            }
        });
    }
});
