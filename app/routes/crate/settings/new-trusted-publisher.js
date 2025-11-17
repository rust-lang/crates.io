import Route from '@ember/routing/route';

export default class NewTrustedPublisherRoute extends Route {
  async model() {
    let crate = this.modelFor('crate');
    return { crate };
  }

  setupController(controller, model) {
    super.setupController(controller, model);

    controller.publisher = 'GitHub';
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
    } else if (repository && repository.startsWith('https://gitlab.com/')) {
      controller.publisher = 'GitLab';
      try {
        let url = new URL(repository);
        let pathParts = url.pathname.slice(1).split('/');

        // Find the repository path end (indicated by /-/ for trees/blobs/etc)
        let repoEndIndex = pathParts.indexOf('-');
        if (repoEndIndex !== -1) {
          pathParts = pathParts.slice(0, repoEndIndex);
        }

        if (pathParts.length >= 2) {
          // For GitLab, support nested groups: https://gitlab.com/a/b/c
          // namespace = "a/b", project = "c"
          controller.namespace = pathParts.slice(0, -1).join('/');
          controller.project = pathParts.at(-1).replace(/.git$/, '');
        }
      } catch {
        // ignore malformed URLs
      }
    }
  }
}
