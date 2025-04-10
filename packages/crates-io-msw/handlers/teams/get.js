import { http, HttpResponse } from 'msw';

import { db } from '../../index.js';
import { serializeTeam } from '../../serializers/team.js';
import { notFound } from '../../utils/handlers.js';

export default http.get('/api/v1/teams/:team_id', ({ params }) => {
  let username = params.team_id;
  let team = db.team.findFirst({ where: { username: { equals: username } } });
  if (!team) {
    return notFound();
  }

  return HttpResponse.json({ team: serializeTeam(team) });
});
