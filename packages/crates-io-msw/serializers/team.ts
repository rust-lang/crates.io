import type { components } from '@crates-io/api-client';
import type { Team } from '../models/index.js';

type ApiTeam = components['schemas']['Team'];

export function serializeTeam(team: Team): ApiTeam {
  return {
    id: team.id,
    login: team.login,
    name: team.name,
    url: team.url,
    avatar: team.avatar,
  };
}
