import ApplicationSerializer from './application';

export default class TrustpubGitLabConfigSerializer extends ApplicationSerializer {
  modelNameFromPayloadKey() {
    return 'trustpub-gitlab-config';
  }

  payloadKeyFromModelName() {
    return 'gitlab_config';
  }
}
