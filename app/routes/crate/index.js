import Ember from 'ember';

export default Ember.Route.extend({
    beforeModel() {
        const url = this.get('router.url');
        if (url.startsWith('/crates/') && !url.endsWith('/')) {
            this.replaceWith('crate.version', '');
        } else {
            this.transitionTo('crate.version', '');
        }
    }
});
