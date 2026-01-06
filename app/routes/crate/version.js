import { NotFoundError } from '@ember-data/adapter/error';
import Route from '@ember/routing/route';
import { service } from '@ember/service';

export default class VersionRoute extends Route {
  @service router;
  @service store;

  async model(params, transition) {
    let crate = this.modelFor('crate');

    let version;
    let requestedVersion = params.version_num;
    let num = requestedVersion || crate.default_version;

    try {
      version =
        crate.loadedVersionsByNum.get(num) ??
        (await crate.store.queryRecord('version', {
          name: crate.id,
          num,
        }));
    } catch (error) {
      if (error instanceof NotFoundError) {
        let title =
          requestedVersion == null
            ? `${crate.name}: Failed to find default version`
            : `${crate.name}: Version ${requestedVersion} not found`;
        return this.router.replaceWith('catch-all', { transition, title });
      } else {
        let title = `${crate.name}: Failed to load version data`;
        return this.router.replaceWith('catch-all', { transition, error, title, tryAgain: true });
      }
    }

    return { crate, requestedVersion, version };
  }

  serialize(model) {
    let version_num = model.num;
    return { version_num };
  }
}
