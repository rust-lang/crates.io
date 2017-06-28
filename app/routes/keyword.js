import Ember from 'ember';

export default Ember.Route.extend({
    flashMessages: Ember.inject.service(),

    model(params) {
        return this.store.find('keyword', params.keyword_id).catch(e => {
            if (e.errors.any(e => e.detail === 'Not Found')) {
                this.get('flashMessages').show(`Keyword '${params.keyword_id}' does not exist`);
            }
        });
    }
});
