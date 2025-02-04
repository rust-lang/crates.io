import { serializeModel } from '../utils/serializers.js';

export function serializeTeam(team) {
  let serialized = serializeModel(team);

  delete serialized.org;

  return serialized;
}
