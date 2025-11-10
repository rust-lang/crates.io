import ApplicationAdapter from './application';

export default class TrustpubGitLabConfigAdapter extends ApplicationAdapter {
  pathForType() {
    return 'trusted_publishing/gitlab_configs';
  }
}
