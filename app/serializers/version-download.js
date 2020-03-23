import ApplicationSerializer from './application';

export default class VersionDownloadSerializer extends ApplicationSerializer {
  extractId(modelClass, resourceHash) {
    return `${resourceHash.date}-${resourceHash.version}`;
  }
}
