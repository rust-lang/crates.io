import { Response } from 'ember-cli-mirage';

import { getSession } from '../utils/session';

export function register(server) {
  server.get('/api/v1/me', function (schema) {
    let { user } = getSession(schema);
    if (!user) {
      return new Response(403, {}, { errors: [{ detail: 'must be logged in to perform that action' }] });
    }

    let json = this.serialize(user);

    // TODO fill this with data from the `schema`
    json.owned_crates = [];

    return json;
  });

  server.put('/api/v1/confirm/:token', (schema, request) => {
    let { token } = request.params;

    let user = schema.users.findBy({ emailVerificationToken: token });
    if (!user) {
      return new Response(400, {}, { errors: [{ detail: 'Email belonging to token not found.' }] });
    }

    user.update({ emailVerified: true, emailVerificationToken: null });

    return { ok: true };
  });
}
