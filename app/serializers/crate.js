import ApplicationSerializer from './application';

const SKIP_NULL_FIELDS = new Set(['categories', 'keywords', 'max_stable_version']);

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
      payload.crates.forEach(crate => removeNullFields(crate));
    }

    return super.normalizeQueryResponse(...arguments);
  }

  normalizeQueryRecordResponse(_store, _modelClass, payload) {
    if (payload.crate) {
      removeNullFields(payload.crate);
    }

    return super.normalizeQueryResponse(...arguments);
  }
}

function removeNullFields(crate) {
  for (let rel of SKIP_NULL_FIELDS) {
    if (crate[rel] === null) {
      delete crate[rel];
    }
  }

  if (crate.max_version == '0.0.0') {
    delete crate.max_version;
  }

  if (crate.newest_version == '0.0.0') {
    delete crate.newest_version;
  }
}
