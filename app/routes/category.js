import Ember from 'ember';

export default Ember.Route.extend({
    flashMessages: Ember.inject.service(),

    model(params) {
        return this.store.find('category', params.category_id).catch(e => {
            if (e.errors.any(e => e.detail === 'Not Found')) {
                this.get('flashMessages').show(`Category '${params.category_id}' does not exist`);
            }
        });
    }
});
