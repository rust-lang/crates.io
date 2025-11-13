import { serializeModel } from '../../utils/serializers.js';

export function serializeGitLabConfig(config) {
  let serialized = serializeModel(config);

  // Extract crate name from the crate relationship
  serialized.crate = serialized.crate.name;

  return serialized;
}
