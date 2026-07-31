import { db } from '../../index.js';
import { notFound } from '../../utils/handlers.js';
import { http } from '../../utils/openapi-http.js';
import { getSession } from '../../utils/session.js';

export default http.put('/api/v1/crates/{name}/owners', async ({ request, params, response }) => {
  let { user } = getSession();
  if (!user) {
    return response.untyped(
      Response.json({ errors: [{ detail: 'must be logged in to perform that action' }] }, { status: 403 }),
    );
  }

  let crate = db.crate.findFirst(q => q.where({ name: params.name }));
  if (!crate) {
    return response.untyped(notFound());
  }

  let body = await request.json();

  let users = [];
  let teams = [];
  let msgs = [];
  for (let login of body.owners) {
    if (login.includes(':')) {
      let team = db.team.findFirst(q => q.where({ login }));
      if (!team) {
        let errorMessage = `could not find team with login \`${login}\``;
        return response.untyped(Response.json({ errors: [{ detail: errorMessage }] }, { status: 404 }));
      }

      teams.push(team);
      msgs.push(`team ${login} has been added as an owner of crate ${crate.name}`);
    } else {
      let user = db.user.findFirst(q => q.where({ login }));
      if (!user) {
        let errorMessage = `could not find user with login \`${login}\``;
        return response.untyped(Response.json({ errors: [{ detail: errorMessage }] }, { status: 404 }));
      }

      users.push(user);
      msgs.push(`user ${login} has been invited to be an owner of crate ${crate.name}`);
    }
  }

  for (let team of teams) {
    await db.crateOwnership.create({ crate, team });
  }

  for (let invitee of users) {
    await db.crateOwnerInvitation.create({ crate, inviter: user, invitee });
  }

  return response(200).json({ ok: true, msg: msgs.join(',') });
});
