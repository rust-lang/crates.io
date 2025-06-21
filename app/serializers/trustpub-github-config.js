import ApplicationSerializer from './application';

export default class TrustpubGitHubConfigSerializer extends ApplicationSerializer {
  modelNameFromPayloadKey() {
    return 'trustpub-github-config';
  }

  payloadKeyFromModelName() {
    return 'github_config';
  }
}
