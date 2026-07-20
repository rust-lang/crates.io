import type { Team } from '../models/index.js';

import { serializeModel } from '../utils/serializers.js';

export function serializeTeam(team: Team) {
  let serialized = serializeModel(team);

  delete serialized.org;

  return serialized;
}
