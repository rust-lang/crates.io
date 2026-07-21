import type { SuccessBody } from '../../utils/api-types.js';

import { http, HttpResponse } from 'msw';

import { db } from '../../index.js';
import { serializeTeam } from '../../serializers/team.js';
import { notFound } from '../../utils/handlers.js';

export default http.get<{ team_id: string }>('/api/v1/teams/:team_id', ({ params }) => {
  let login = params.team_id;
  let team = db.team.findFirst(q => q.where({ login }));
  if (!team) {
    return notFound();
  }

  return HttpResponse.json<SuccessBody<'find_team'>>({ team: serializeTeam(team) });
});
