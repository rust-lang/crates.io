import Ember from 'ember';

export default Ember.Route.extend({
    queryParams: {
        page: { refreshModel: true },
        sort: { refreshModel: true },
    },

    data: {},

    model(params) {
        const { user_id } = params;
        return this.store.find('user', user_id).then(
            (user) => {
                params.user_id = user.get('id');
                return Ember.RSVP.hash({
                    crates: this.store.query('crate', params),
                    user
                });
            },
            (e) => {
                if (e.errors.any(e => e.detail === 'Not Found')) {
                    this
                        .controllerFor('application')
                        .set('nextFlashError', `User '${params.user_id}' does not exist`);
                    return this.replaceWith('index');
                }
            }
        );
    },

    setupController(controller, model) {
        this._super(controller, model);

        controller.set('fetchingFeed', true);
        controller.set('crates', this.get('data.crates'));
    },
});
