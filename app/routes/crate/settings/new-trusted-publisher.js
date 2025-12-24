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
    if (!repository) return;

    let prefillData = parseRepositoryUrl(repository);
    if (prefillData) {
      controller.publisher = prefillData.publisher;
      controller.namespace = prefillData.namespace;
      controller.project = prefillData.project;
    }
  }
}

function parseRepositoryUrl(repository) {
  if (repository.startsWith('https://github.com/')) {
    return parseGitHubUrl(repository);
  } else if (repository.startsWith('https://gitlab.com/')) {
    return parseGitLabUrl(repository);
  }
  return null;
}

function parseGitHubUrl(repository) {
  try {
    let url = new URL(repository);
    let pathParts = url.pathname.slice(1).split('/');
    if (pathParts.length >= 2) {
      return {
        publisher: 'GitHub',
        namespace: pathParts[0],
        project: pathParts[1].replace(/.git$/, ''),
      };
    }
  } catch {
    // ignore malformed URLs
  }
  return null;
}

function parseGitLabUrl(repository) {
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
      return {
        publisher: 'GitLab',
        namespace: pathParts.slice(0, -1).join('/'),
        project: pathParts.at(-1).replace(/.git$/, ''),
      };
    }
  } catch {
    // ignore malformed URLs
  }
  return null;
}
