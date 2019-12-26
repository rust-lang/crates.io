import ApplicationSerializer from './application';

export default ApplicationSerializer.extend({
  isNewSerializerAPI: true,

  extractRelationships(modelClass, resourceHash) {
    if (resourceHash.versions == null) {
      delete resourceHash.versions;
    }

    return this._super(...arguments);
  },
});
