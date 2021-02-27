import Route from '@ember/routing/route';

export default class VersionRoute extends Route {
  async model() {
    let crate = this.modelFor('crate');
    let versions = await crate.get('versions');

    let { defaultVersion } = crate;
    let version = versions.find(version => version.num === defaultVersion) ?? versions.lastObject;

    this.replaceWith('crate.version-dependencies', crate, version.num);
  }
}
