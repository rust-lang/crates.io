import ApplicationSerializer from './application';

export default class CrateSerializer extends ApplicationSerializer {
  isNewSerializerAPI = true;

  extractRelationships(modelClass, resourceHash) {
    if (resourceHash.versions == null) {
      delete resourceHash.versions;
    }

    return super.extractRelationships(...arguments);
  }
}
