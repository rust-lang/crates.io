import type { TrustpubGitlabConfig } from '../../models/index.js';

import { serializeModel } from '../../utils/serializers.js';

export function serializeGitLabConfig(config: TrustpubGitlabConfig) {
  let serialized = serializeModel(config);

  // Extract crate name from the crate relationship
  serialized.crate = serialized.crate.name;

  return serialized;
}
