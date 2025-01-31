import { serializeModel } from '../utils/serializers.js';

export function serializeDependency(dependency) {
  let serialized = serializeModel(dependency);

  serialized.crate_id = dependency.crate.name;
  serialized.version_id = dependency.version.id;

  delete serialized.crate;
  delete serialized.version;

  return serialized;
}
