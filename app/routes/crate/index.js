import Ember from 'ember';

export default Ember.Route.extend({
    redirect(model, transition) {
        if (transition.intent.url) {
            this.replaceWith('crate.version', '');
        } else {
            this.transitionTo('crate.version', '');
        }
    }
});
