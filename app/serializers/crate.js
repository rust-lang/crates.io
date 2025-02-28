import ApplicationSerializer from './application';

const SKIP_NULL_FIELDS = new Set(['categories', 'keywords']);

export default class CrateSerializer extends ApplicationSerializer {
  isNewSerializerAPI = true;

  extractRelationships(modelClass, resourceHash) {
    if (resourceHash.versions == null) {
      delete resourceHash.versions;
    }

    return super.extractRelationships(...arguments);
  }

  normalizeQueryResponse(_store, _modelClass, payload) {
    // We don't want existing relationships overwritten by results with null values.
    // See: https://github.com/rust-lang/crates.io/issues/10711
    if (payload.crates) {
      payload.crates = payload.crates.map(crate => {
        for (const rel of SKIP_NULL_FIELDS) {
          if (crate[rel] === null) {
            delete crate[rel];
          }
        }
        return crate;
      });
    }
    return super.normalizeQueryResponse(...arguments);
  }
}
