import Route from '@ember/routing/route';
import { inject as service } from '@ember/service';

export default class VersionRoute extends Route {
  @service notifications;
  @service router;

  async model(params) {
    let crate = this.modelFor('crate');
    let versions = await crate.get('versions');

    let requestedVersion = params.version_num;
    let version = versions.find(version => version.num === requestedVersion);
    if (!version) {
      this.notifications.error(`Version '${requestedVersion}' of crate '${crate.name}' does not exist`);
      this.router.replaceWith('crate.index');
    }

    try {
      await version.loadDepsTask.perform();
    } catch {
      this.notifications.error(
        `Failed to load the list of dependencies for the '${crate.name}' crate. Please try again later!`,
      );
      this.router.replaceWith('crate.index');
    }

    return version;
  }

  setupController(controller, model) {
    controller.set('version', model);
    controller.set('crate', this.modelFor('crate'));
  }
}
