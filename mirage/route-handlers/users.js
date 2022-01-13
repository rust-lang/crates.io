import { Response } from 'miragejs';

import { getSession } from '../utils/session';
import { notFound } from './-utils';

export function register(server) {
  server.get('/api/v1/users/:user_id', (schema, request) => {
    let login = request.params.user_id;
    let user = schema.users.findBy({ login });
    return user ? user : notFound();
  });

  server.put('/api/v1/users/:user_id', (schema, request) => {
    let { user } = getSession(schema);
    if (!user) {
      // unfortunately, it's hard to tell from the Rust code if this is the correct response
      // in this case, but since it's used elsewhere I will assume for now that it's correct.
      return new Response(403, {}, { errors: [{ detail: 'must be logged in to perform that action' }] });
    }

    if (user.id !== request.params.user_id) {
      return new Response(400, {}, { errors: [{ detail: 'current user does not match requested user' }] });
    }

    let json = JSON.parse(request.requestBody);
    if (!json || !json.user || !('email' in json.user)) {
      return new Response(400, {}, { errors: [{ detail: 'invalid json request' }] });
    }
    if (!json.user.email) {
      return new Response(400, {}, { errors: [{ detail: 'empty email rejected' }] });
    }

    user.update({
      email: json.user.email,
      emailVerified: false,
      emailVerificationToken: 'secret123',
    });

    return { ok: true };
  });

  server.put('/api/v1/users/:user_id/resend', (schema, request) => {
    let { user } = getSession(schema);
    if (!user) {
      // unfortunately, it's hard to tell from the Rust code if this is the correct response
      // in this case, but since it's used elsewhere I will assume for now that it's correct.
      return new Response(403, {}, { errors: [{ detail: 'must be logged in to perform that action' }] });
    }

    if (user.id !== request.params.user_id) {
      return new Response(400, {}, { errors: [{ detail: 'current user does not match requested user' }] });
    }

    // let's pretend that we're sending an email here... :D

    return { ok: true };
  });
}
