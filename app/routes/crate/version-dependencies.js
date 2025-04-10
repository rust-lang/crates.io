import { NotFoundError } from '@ember-data/adapter/error';
import Route from '@ember/routing/route';
import { service } from '@ember/service';

export default class VersionRoute extends Route {
  @service store;
  @service router;

  async model(params, transition) {
    let crate = this.modelFor('crate');

    let requestedVersion = params.version_num;
    let version;
    try {
      version =
        crate.loadedVersionsByNum.get(requestedVersion) ??
        (await this.store.queryRecord('version', {
          name: crate.id,
          num: requestedVersion,
        }));
    } catch (error) {
      if (error instanceof NotFoundError) {
        let title = `${crate.name}: Version ${requestedVersion} not found`;
        return this.router.replaceWith('catch-all', { transition, title });
      } else {
        let title = `${crate.name}: Failed to load version data`;
        return this.router.replaceWith('catch-all', { transition, error, title, tryAgain: true });
      }
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
