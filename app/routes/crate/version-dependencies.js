import Route from '@ember/routing/route';
import { inject as service } from '@ember/service';

export default class VersionRoute extends Route {
  @service router;

  async model(params, transition) {
    let crate = this.modelFor('crate');

    let versions;
    try {
      versions = await crate.loadVersionsTask.perform();
    } catch (error) {
      let title = `${crate.name}: Failed to load version data`;
      return this.router.replaceWith('catch-all', { transition, error, title, tryAgain: true });
    }

    let requestedVersion = params.version_num;
    let version = versions.find(version => version.num === requestedVersion);
    if (!version) {
      let title = `${crate.name}: Version ${requestedVersion} not found`;
      return this.router.replaceWith('catch-all', { transition, title });
    }

    try {
      await version.loadDepsTask.perform();
    } catch (error) {
      let title = `${crate.name}: Failed to load dependencies`;
      return this.router.replaceWith('catch-all', { transition, error, title, tryAgain: true });
    }

    return version;
  }

  setupController(controller, model) {
    controller.set('version', model);
    controller.set('crate', this.modelFor('crate'));
  }
}
