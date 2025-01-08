import Route from '@ember/routing/route';
import { inject as service } from '@ember/service';

export default class VersionRoute extends Route {
  @service router;

  async model() {
    let crate = this.modelFor('crate');
    let { default_version } = crate;

    this.router.replaceWith('crate.version-dependencies', crate, default_version);
  }
}
