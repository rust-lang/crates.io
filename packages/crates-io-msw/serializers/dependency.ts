import type { Dependency } from '../models/index.js';

import { serializeModel } from '../utils/serializers.js';

export function serializeDependency(dependency: Dependency) {
  let serialized = serializeModel(dependency);

  serialized.crate_id = dependency.crate.name;
  serialized.version_id = dependency.version.id;
  serialized.downloads = dependency.crate.downloads;

  delete serialized.crate;
  delete serialized.version;

  return serialized;
}
