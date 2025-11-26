import Route from '@ember/routing/route';
import { service } from '@ember/service';

export default class SettingsIndexRoute extends Route {
  @service store;

  async model() {
    let crate = this.modelFor('crate');

    let githubConfigs = await this.store.query('trustpub-github-config', { crate: crate.name });
    let gitlabConfigs = await this.store.query('trustpub-gitlab-config', { crate: crate.name });

    return { crate, githubConfigs, gitlabConfigs };
  }

  setupController(controller, { crate, githubConfigs, gitlabConfigs }) {
    super.setupController(...arguments);

    controller.set('crate', crate);
    controller.set('githubConfigs', githubConfigs);
    controller.set('gitlabConfigs', gitlabConfigs);

    // Capture whether the trustpub_only checkbox should be visible on initial load
    let hasConfigs = githubConfigs?.length > 0 || gitlabConfigs?.length > 0;
    controller.set('trustpubOnlyCheckboxWasVisible', hasConfigs || crate.trustpub_only);
  }
}
