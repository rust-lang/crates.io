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
}
