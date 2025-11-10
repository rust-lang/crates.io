import Route from '@ember/routing/route';

export default class NewTrustedPublisherRoute extends Route {
  async model() {
    let crate = this.modelFor('crate');
    return { crate };
  }

  setupController(controller, model) {
    super.setupController(controller, model);

    controller.namespace = '';
    controller.project = '';
    controller.workflow = '';
    controller.environment = '';

    let repository = model.crate.repository;
    if (repository && repository.startsWith('https://github.com/')) {
      try {
        let url = new URL(repository);
        let pathParts = url.pathname.slice(1).split('/');
        if (pathParts.length >= 2) {
          controller.namespace = pathParts[0];
          controller.project = pathParts[1].replace(/.git$/, '');
        }
      } catch {
        // ignore malformed URLs
      }
    }
  }
}
