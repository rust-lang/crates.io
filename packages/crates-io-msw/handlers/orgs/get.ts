import { db } from '../../index.js';
import { serializeTeam } from '../../serializers/team.js';
import { http } from '../../utils/openapi-http.js';

export default http.get('/api/v1/orgs/{org}', ({ params, response }) => {
  let teams = db.team.findMany(q => q.where({ login: login => login.startsWith(params.org) }));

  return response(200).json({ teams: teams.map(team => serializeTeam(team)) });
});
