import Ember from 'ember';
import FastBootUtils from 'cargo/mixins/fastboot-utils';

const { inject: { service } } = Ember;

export default Ember.Route.extend(FastBootUtils, {

    ajax: service(),

    activate() {
        this.get('ajax').request(`${this.get('appURL')}/logout`).then(() => {
            Ember.run(() => {
                this.session.logoutUser();
                this.transitionTo('index');
            });
        });
    }
});
