import ApplicationSerializer from './application';

export default ApplicationSerializer.extend({
  extractId(modelClass, resourceHash) {
    return `${resourceHash.date}-${resourceHash.version}`;
  },
});
