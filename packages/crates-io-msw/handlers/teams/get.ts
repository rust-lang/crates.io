import { db } from '../../index.js';
import { serializeTeam } from '../../serializers/team.js';
import { notFoundError } from '../../utils/handlers.js';
import { http } from '../../utils/openapi-http.js';

export default http.get('/api/v1/teams/{team}', ({ params, response }) => {
  let team = db.team.findFirst(q => q.where({ login: params.team }));
  if (!team) {
    return response('4XX').json(notFoundError(), { status: 404 });
  }

  return response(200).json({ team: serializeTeam(team) });
});
