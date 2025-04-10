import { http, HttpResponse } from 'msw';

import { db } from '../../index.js';
import { notFound } from '../../utils/handlers.js';
import { getSession } from '../../utils/session.js';

export default http.put('/api/v1/crates/:name/owners', async ({ request, params }) => {
  let { user } = getSession();
  if (!user) {
    return HttpResponse.json({ errors: [{ detail: 'must be logged in to perform that action' }] }, { status: 403 });
  }

  let crate = db.crate.findFirst({ where: { name: { equals: params.name } } });
  if (!crate) {
    return notFound();
  }

  let body = await request.json();

  let users = [];
  let teams = [];
  let msgs = [];
  for (let username of body.owners) {
    if (username.includes(':')) {
      let team = db.team.findFirst({ where: { username: { equals: username } } });
      if (!team) {
        let errorMessage = `could not find team with username \`${username}\``;
        return HttpResponse.json({ errors: [{ detail: errorMessage }] }, { status: 404 });
      }

      teams.push(team);
      msgs.push(`team ${username} has been added as an owner of crate ${crate.name}`);
    } else {
      let user = db.user.findFirst({ where: { username: { equals: username } } });
      if (!user) {
        let errorMessage = `could not find user with username \`${username}\``;
        return HttpResponse.json({ errors: [{ detail: errorMessage }] }, { status: 404 });
      }

      users.push(user);
      msgs.push(`user ${username} has been invited to be an owner of crate ${crate.name}`);
    }
  }

  for (let team of teams) {
    db.crateOwnership.create({ crate, team });
  }

  for (let invitee of users) {
    db.crateOwnerInvitation.create({ crate, inviter: user, invitee });
  }

  return HttpResponse.json({ ok: true, msg: msgs.join(',') });
});
