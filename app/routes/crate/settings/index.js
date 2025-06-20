import Route from '@ember/routing/route';
import { service } from '@ember/service';

export default class SettingsIndexRoute extends Route {
  @service store;

  async model() {
    let crate = this.modelFor('crate');

    let githubConfigs = await this.store.query('trustpub-github-config', { crate: crate.name });

    return { crate, githubConfigs };
  }

  setupController(controller, { crate, githubConfigs }) {
    super.setupController(...arguments);

    controller.set('crate', crate);
    controller.set('githubConfigs', githubConfigs);
  }
}
