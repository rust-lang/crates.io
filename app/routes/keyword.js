import Ember from 'ember';

export default Ember.Route.extend({
    model(params) {
        return this.store.find('keyword', params.keyword_id).catch(e => {
            if (e.errors.any(e => e.detail === 'Not Found')) {
                this.controllerFor('application').set('flashError', `Keyword '${params.keyword_id}' does not exist`);
            }
        });
    }
});
