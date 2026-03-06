import { http, HttpResponse } from 'msw';

import { db } from '../../index.js';
import { notFound } from '../../utils/handlers.js';

export default http.get('/api/v1/crates/:name/owners', async ({ params }) => {
  let crate = db.crate.findFirst(q => q.where({ name: params.name }));
  if (!crate) {
    return notFound();
  }

  let ownerships = db.crateOwnership.findMany(q => q.where(ownership => ownership.crate.id === crate.id));

  let users = [
    ...ownerships
      .filter(o => o.user)
      .map(o => ({
        id: o.user.id,
        login: o.user.login,
        kind: 'user',
        url: `https://github.com/${o.user.login}`,
        name: o.user.name,
        avatar: o.user.avatar,
      })),
    ...ownerships
      .filter(o => o.team)
      .map(o => ({
        id: o.team.id,
        login: o.team.login,
        kind: 'team',
        url: o.team.url,
        name: o.team.name,
        avatar: o.team.avatar,
      })),
  ];

  return HttpResponse.json({ users });
});
