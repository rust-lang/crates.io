import Ember from 'ember';

export default Ember.Route.extend({
    data: {},

    setupController(controller, model) {
        this._super(controller, model);

        controller.set('fetchingFeed', true);
        controller.set('crates', this.get('data.crates'));
    },

    model(params) {
        return this.store.find('user', params.user_id).catch(e => {
            if (e.errors.any(e => e.detail === 'Not Found')) {
                this.controllerFor('application').set('nextFlashError', `User '${params.user_id}' does not exist`);
                return this.replaceWith('index');
            }
        });
    },

    afterModel(user) {
        let crates = this.store.query('crate', {
            user_id: user.get('id')
        });

        return Ember.RSVP.hash({
            crates,
        }).then((hash) => this.set('data', hash));
    }
});
