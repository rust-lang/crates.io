import type { components } from '@crates-io/api-client';

import { db } from '../../index.js';
import { serializeUser } from '../../serializers/user.js';
import { notFoundError } from '../../utils/handlers.js';
import { http } from '../../utils/openapi-http.js';

type Owner = components['schemas']['Owner'];
type UserOwner = Extract<Owner, { kind: 'user' }>;
type TeamOwner = Extract<Owner, { kind: 'team' }>;

export default http.get('/api/v1/crates/{name}/owners', ({ params, response }) => {
  let crate = db.crate.findFirst(q => q.where({ name: params.name }));
  if (!crate) {
    return response('4XX').json(notFoundError(), { status: 404 });
  }

  let ownerships = db.crateOwnership.findMany(q => q.where(ownership => ownership.crate.id === crate.id));

  let users = [
    ...ownerships
      .filter(o => o.user)
      .map((o): UserOwner => {
        let user = serializeUser(o.user);
        return { ...user, name: user.name ?? null, avatar: user.avatar ?? null, kind: 'user' };
      }),
    ...ownerships
      .filter(o => o.team)
      .map((o): TeamOwner => ({
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
