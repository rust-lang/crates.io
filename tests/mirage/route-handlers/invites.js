import { Response } from 'miragejs';

import { getSession } from '../utils/session';
import { notFound } from './-utils';

export function register(server) {
  server.get('/api/private/crate_owner_invitations', function (schema, request) {
    let { user } = getSession(schema);
    if (!user) {
      return new Response(403, {}, { errors: [{ detail: 'must be logged in to perform that action' }] });
    }

    let invites;
    if (request.queryParams['crate_name']) {
      let crate = schema.crates.findBy({ name: request.queryParams['crate_name'] });
      if (!crate) return notFound();

      invites = schema.crateOwnerInvitations.where({ crateId: crate.id });
    } else if (request.queryParams['invitee_id']) {
      let inviteeId = request.queryParams['invitee_id'];
      if (inviteeId !== user.id) {
        return new Response(403, {}, { errors: [{ detail: 'must be logged in to perform that action' }] });
      }

      invites = schema.crateOwnerInvitations.where({ inviteeId });
    } else {
      return new Response(400, {}, { errors: [{ detail: 'missing or invalid filter' }] });
    }

    let perPage = 10;
    let start = request.queryParams['__start__'] ?? 0;
    let end = start + perPage;

    let nextPage = null;
    if (invites.length > end) {
      let url = new URL(request.url, 'https://crates.io');
      url.searchParams.set('__start__', end);
      nextPage = url.search;
    }

    invites = invites.slice(start, end);

    let response = this.serialize(invites);
    response.users ??= [];
    response.meta = { next_page: nextPage };

    return response;
  });
}
