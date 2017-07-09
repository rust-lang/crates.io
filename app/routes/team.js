import RSVP from 'rsvp';
import { inject as service } from '@ember/service';
import Route from '@ember/routing/route';

export default Route.extend({
    flashMessages: service(),

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

        return this.store.queryRecord('team', { team_id }).then(
            (team) => {
                params.team_id = team.get('id');
                return RSVP.hash({
                    crates: this.store.query('crate', params),
                    team
                });
            },
            (e) => {
                if (e.errors.any(e => e.detail === 'Not Found')) {
                    this.get('flashMessages').queue(`User '${params.team_id}' does not exist`);
                    return this.replaceWith('index');
                }
            }
        );
    },
});
