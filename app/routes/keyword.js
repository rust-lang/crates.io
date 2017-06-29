import Ember from 'ember';

export default Ember.Route.extend({
    flashMessages: Ember.inject.service(),

    model({ keyword_id }) {
        return this.store.find('keyword', keyword_id).catch(e => {
            if (e.errors.any(e => e.detail === 'Not Found')) {
                this.get('flashMessages').show(`Keyword '${keyword_id}' does not exist`);
            }
        });
    }
});
