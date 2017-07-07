import Ember from 'ember';

const { inject: { service } } = Ember;

export default Ember.Route.extend({

    ajax: service(),

    activate() {
        this.get('ajax').request(`/logout`).then(() => {
            Ember.run(() => {
                this.session.logoutUser();
                this.transitionTo('index');
            });
        });
    }
});
