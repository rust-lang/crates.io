import Ember from 'ember';

export default Ember.Route.extend({
    queryParams: {
        page: { refreshedModel: true },
        sort: { refreshedModel: true },
    },
    data: {},

    setupController(controller, model) {
        this._super(controller, model);

        controller.set('fetchingFeed', true);
        controller.set('crates', this.get('data.crates'));
    },

    model(params) {
        const { team_id } = params;

        return this.store.find('team', team_id).then(
            (team) => {
                params.team_id = team.get('id');
                return Ember.RSVP.hash({
                    crates: this.store.query('crate', params),
                    team
                });
            },
            (e) => {
                if (e.errors.any(e => e.detail === 'Not Found')) {
                    this
                        .controllerFor('application')
                        .set('nextFlashError', `User '${params.team_id}' does not exist`);
                    return this.replaceWith('index');
                }
            }
        );
    },
});
