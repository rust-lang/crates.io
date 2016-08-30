import Ember from 'ember';

export default Ember.Route.extend({
    model(params) {
        return this.store.find('crate', params.crate_id).catch(e => {
            if (e.errors.any(e => e.detail === 'Not Found')) {
                throw new Error(`${params.crate_id} not found`);
            }
        });
    }
});
