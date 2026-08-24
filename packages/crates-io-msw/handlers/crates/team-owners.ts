import { db } from '../../index.js';
import { serializeTeam } from '../../serializers/team.js';
import { notFoundError } from '../../utils/handlers.js';
import { http } from '../../utils/openapi-http.js';

export default http.get('/api/v1/crates/{name}/owner_team', ({ params, response }) => {
  let crate = db.crate.findFirst(q => q.where({ name: params.name }));
  if (!crate) {
    return response('4XX').json(notFoundError(), { status: 404 });
  }

  let ownerships = db.crateOwnership.findMany(q => q.where(ownership => ownership.crate.id === crate.id));

  return response(200).json({
    teams: ownerships
      .filter(o => o.team)
      .map(o => {
        let team = serializeTeam(o.team);
        return {
          ...team,
          name: team.name ?? null,
          avatar: team.avatar ?? null,
          url: team.url ?? null,
          kind: 'team',
        };
      }),
  });
});
