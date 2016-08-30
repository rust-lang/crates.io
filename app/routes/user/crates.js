import Ember from 'ember';

export default Ember.Route.extend({
    queryParams: {
        page: { refreshModel: true },
        sort: { refreshModel: true },
    },

    model(params) {
        const { user_id } = this.paramsFor('user');
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
});

