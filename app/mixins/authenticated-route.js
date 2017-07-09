import Mixin from '@ember/object/mixin';
import { inject as service } from '@ember/service';
import $ from 'jquery';

export default Mixin.create({
    flashMessages: service(),

    beforeModel(transition) {
        let user = this.session.get('currentUser');
        if (user !== null) {
            return;
        }

        // The current user is loaded asynchronously, so if we haven't actually
        // loaded the current user yet then we need to wait for it to be loaded.
        // Once we've done that we can retry the transition and start the whole
        // process over again!
        if (!window.currentUserDetected) {
            transition.abort();
            $(window).on('currentUserDetected', function() {
                $(window).off('currentUserDetected');
                transition.retry();
            });
        } else {
            this.session.set('savedTransition', transition);
            this.get('flashMessages').queue('Please log in to proceed');
            return this.transitionTo('index');
        }
    },
});
