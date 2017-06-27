import Ember from 'ember';
import fetch from 'fetch';
import FastbootUtils from '../mixins/fastboot-utils';

export default Ember.Route.extend(FastbootUtils, {

    activate() {
        fetch(`${this.get('appURL')}/logout`, () => {
            Ember.run(() => {
                this.session.logoutUser();
                this.transitionTo('index');
            });
        });
    }
});
