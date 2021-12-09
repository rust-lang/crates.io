import Route from '@ember/routing/route';
import { inject as service } from '@ember/service';

export default class VersionRoute extends Route {
  @service router;

  async model() {
    let crate = this.modelFor('crate');
    let versions = await crate.get('versions');

    let { defaultVersion } = crate;
    let version = versions.find(version => version.num === defaultVersion) ?? versions.lastObject;

    this.router.replaceWith('crate.version-dependencies', crate, version.num);
  }
}
