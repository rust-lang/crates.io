import { db } from '../../index.js';
import { notFound } from '../../utils/handlers.js';
import { http } from '../../utils/openapi-http.js';

export default http.get('/api/v1/crates/{name}/owners', ({ params, response }) => {
  let crate = db.crate.findFirst(q => q.where({ name: params.name }));
  if (!crate) {
    return response.untyped(notFound());
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

  return response(200).json({ users });
});
