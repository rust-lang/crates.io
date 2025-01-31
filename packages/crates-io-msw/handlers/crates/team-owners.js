import { http, HttpResponse } from 'msw';

import { db } from '../../index.js';
import { serializeTeam } from '../../serializers/team.js';
import { notFound } from '../../utils/handlers.js';

export default http.get('/api/v1/crates/:name/owner_team', async ({ params }) => {
  let crate = db.crate.findFirst({ where: { name: { equals: params.name } } });
  if (!crate) {
    return notFound();
  }

  let ownerships = db.crateOwnership.findMany({ where: { crate: { id: { equals: crate.id } } } });

  return HttpResponse.json({
    teams: ownerships.filter(o => o.team).map(o => ({ ...serializeTeam(o.team), kind: 'team' })),
  });
});
