import Route from '@ember/routing/route';

export default class SettingsRoute extends Route {
  async model() {
    let crate = this.modelFor('crate');
    let [users, teams] = await Promise.all([crate.owner_user, crate.owner_team]);
    return { crate, teams, users };
  }
}
