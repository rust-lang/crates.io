import Ember from 'ember';

export default Ember.Mixin.create({
    beforeModel: function(transition) {
        var user = this.session.get('currentUser');
        if (user === null) {
            this.session.set('savedTransition', transition);
            this.controllerFor('application').set('nextFlashError',
                                                  'Please log in to proceed');
            return this.transitionTo('index');
        }
    },
});
