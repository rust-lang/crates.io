import { Response } from 'ember-cli-mirage';

import { getSession } from '../utils/session';

export function register(server) {
  server.get('/api/v1/me', function (schema) {
    let { user } = getSession(schema);
    if (!user) {
      return new Response(403, {}, { errors: [{ detail: 'must be logged in to perform that action' }] });
    }

    let ownerships = schema.crateOwnerships.where({ userId: user.id }).models;

    let json = this.serialize(user);

    json.owned_crates = ownerships.map(ownership => ({
      id: ownership.crate.id,
      name: ownership.crate.name,
      email_notifications: ownership.emailNotifications,
    }));

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
