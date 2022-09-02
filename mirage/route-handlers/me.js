import { Response } from 'miragejs';

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

  server.get('/api/v1/me/tokens', function (schema) {
    let { user } = getSession(schema);
    if (!user) {
      return new Response(403, {}, { errors: [{ detail: 'must be logged in to perform that action' }] });
    }

    return schema.apiTokens.where({ userId: user.id }).sort((a, b) => Number(b.id) - Number(a.id));
  });

  server.put('/api/v1/me/tokens', function (schema) {
    let { user } = getSession(schema);
    if (!user) {
      return new Response(403, {}, { errors: [{ detail: 'must be logged in to perform that action' }] });
    }

    let { name } = this.normalizedRequestAttrs('api-token');
    let token = server.create('api-token', { user, name, createdAt: new Date().toISOString() });

    let json = this.serialize(token);
    json.api_token.revoked = false;
    json.api_token.token = token.token;
    return json;
  });

  server.delete('/api/v1/me/tokens/:tokenId', function (schema, request) {
    let { user } = getSession(schema);
    if (!user) {
      return new Response(403, {}, { errors: [{ detail: 'must be logged in to perform that action' }] });
    }

    let { tokenId } = request.params;
    let token = schema.apiTokens.findBy({ id: tokenId, userId: user.id });
    if (token) token.destroy();

    return {};
  });

  server.get('/api/v1/me/updates', function (schema, request) {
    let { user } = getSession(schema);
    if (!user) {
      return new Response(403, {}, { errors: [{ detail: 'must be logged in to perform that action' }] });
    }

    let allVersions = schema.versions
      .all()
      .filter(version => user.followedCrates.includes(version.crate))
      .sort((a, b) => Number(b.id) - Number(a.id));

    let page = Number(request.queryParams.page) || 1;
    let perPage = 10;

    let begin = (page - 1) * perPage;
    let end = begin + perPage;

    let versions = allVersions.slice(begin, end);

    let totalCount = allVersions.length;
    let totalPages = Math.ceil(totalCount / perPage);
    let more = page < totalPages;

    return { ...this.serialize(versions), meta: { more } };
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

  server.get('/api/v1/me/crate_owner_invitations', function (schema) {
    let { user } = getSession(schema);
    if (!user) {
      return new Response(403, {}, { errors: [{ detail: 'must be logged in to perform that action' }] });
    }

    return schema.crateOwnerInvitations.where({ inviteeId: user.id });
  });

  server.put('/api/v1/me/crate_owner_invitations/:crate_id', (schema, request) => {
    let { user } = getSession(schema);
    if (!user) {
      return new Response(403, {}, { errors: [{ detail: 'must be logged in to perform that action' }] });
    }

    let body = JSON.parse(request.requestBody);
    let { accepted, crate_id: crateId } = body.crate_owner_invite;

    let invite = schema.crateOwnerInvitations.findBy({ crateId, inviteeId: user.id });
    if (!invite) {
      return new Response(404);
    }

    if (accepted) {
      server.create('crate-ownership', { crate: invite.crate, user });
    }

    invite.destroy();

    return { crate_owner_invitation: { crate_id: crateId, accepted } };
  });

  server.put('/api/v1/me/crate_owner_invitations/accept/:token', (schema, request) => {
    let { token } = request.params;

    let invite = schema.crateOwnerInvitations.findBy({ token });
    if (!invite) {
      return new Response(404);
    }

    server.create('crate-ownership', { crate: invite.crate, user: invite.invitee });
    invite.destroy();

    return { crate_owner_invitation: { crate_id: invite.crateId, accepted: true } };
  });
}
