import Ember from 'ember';

export default Ember.Mixin.create({
    beforeModel(transition) {
        var user = this.session.get('currentUser');
        if (user !== null) {
            return;
        }

        // The current user is loaded asynchronously, so if we haven't actually
        // loaded the current user yet then we need to wait for it to be loaded.
        // Once we've done that we can retry the transition and start the whole
        // process over again!
        if (!window.currentUserDetected) {
            transition.abort();
            Ember.$(window).on('currentUserDetected', function() {
                Ember.$(window).off('currentUserDetected');
                transition.retry();
            });
        } else {
            this.session.set('savedTransition', transition);
            this.controllerFor('application').set('nextFlashError',
                                                  'Please log in to proceed');
            return this.transitionTo('index');
        }
    },
});
