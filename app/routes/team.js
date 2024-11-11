import { NotFoundError } from '@ember-data/adapter/error';
import Route from '@ember/routing/route';
import { inject as service } from '@ember/service';

export default class TeamRoute extends Route {
  @service notifications;
  @service router;
  @service store;

  queryParams = {
    page: { refreshModel: true },
    sort: { refreshModel: true },
  };

  async model(params, transition) {
    const { team_id } = params;

    try {
      let team = await this.store.queryRecord('team', { team_id });

      params.team_id = team.get('id');
      params.include_yanked = 'n';
      let crates = await this.store.query('crate', params);

      return { crates, team };
    } catch (error) {
      if (error instanceof NotFoundError) {
        let title = `${team_id}: Team not found`;
        this.router.replaceWith('catch-all', { transition, error, title });
      } else {
        let title = `${team_id}: Failed to load team data`;
        this.router.replaceWith('catch-all', { transition, error, title, tryAgain: true });
      }
    }
  }
}
