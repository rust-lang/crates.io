import ApplicationAdapter from './application';

export default class TrustpubGitHubConfigAdapter extends ApplicationAdapter {
  pathForType() {
    return 'trusted_publishing/github_configs';
  }
}
