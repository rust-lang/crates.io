import { getSession } from '../utils/session';

export function register(server) {
  server.del('/api/private/session', function (schema) {
    let { session } = getSession(schema);
    if (session) {
      session.destroy();
    }

    return { ok: true };
  });
}
