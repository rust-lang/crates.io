import { Response } from 'miragejs';

import { getSession } from '../utils/session';
import { compareIsoDates, compareStrings, notFound, pageParams } from './-utils';

export function list(schema, request) {
  const { start, end } = pageParams(request);

  let crates = schema.crates.all();

  if (request.queryParams.following === '1') {
    let { user } = getSession(schema);
    if (!user) {
      return new Response(403, {}, { errors: [{ detail: 'must be logged in to perform that action' }] });
    }

    crates = user.followedCrates;
  }

  if (request.queryParams.letter) {
    let letter = request.queryParams.letter.toLowerCase();
    crates = crates.filter(crate => crate.name[0].toLowerCase() === letter);
  }

  if (request.queryParams.q) {
    let q = request.queryParams.q.toLowerCase();
    crates = crates.filter(crate => crate.name.toLowerCase().includes(q));
  }

  if (request.queryParams.user_id) {
    let userId = parseInt(request.queryParams.user_id, 10);
    crates = crates.filter(crate => schema.crateOwnerships.findBy({ crateId: crate.id, userId }));
  }

  if (request.queryParams.team_id) {
    let teamId = parseInt(request.queryParams.team_id, 10);
    crates = crates.filter(crate => schema.crateOwnerships.findBy({ crateId: crate.id, teamId }));
  }

  let { ids } = request.queryParams;
  if (ids) {
    crates = crates.filter(crate => ids.includes(crate.name));
  }

  if (request.queryParams.sort === 'alpha') {
    crates = crates.sort((a, b) => compareStrings(a.id.toLowerCase(), b.id.toLowerCase()));
  } else if (request.queryParams.sort === 'recent-downloads') {
    crates = crates.sort((a, b) => b.recent_downloads - a.recent_downloads);
  }

  return { ...this.serialize(crates.slice(start, end)), meta: { total: crates.length } };
}

export function register(server) {
  server.get('/api/v1/crates', list);

  server.get('/api/v1/crates/:name', function (schema, request) {
    let { name } = request.params;
    let crate = schema.crates.findBy({ name });
    if (!crate) return notFound();

    return {
      ...this.serialize(crate),
      ...this.serialize(crate.categories),
      ...this.serialize(crate.keywords),
      ...this.serialize(crate.versions.sort((a, b) => Number(b.id) - Number(a.id))),
    };
  });

  server.delete('/api/v1/crates/:name', (schema, request) => {
    let { user } = getSession(schema);
    if (!user) {
      return new Response(403, {}, { errors: [{ detail: 'must be logged in to perform that action' }] });
    }

    let { name } = request.params;
    let crate = schema.crates.findBy({ name });
    if (!crate) {
      return new Response(404, {}, { errors: [{ detail: `crate \`${name}\` does not exist` }] });
    }

    crate.destroy();

    return '';
  });

  server.get('/api/v1/crates/:name/following', (schema, request) => {
    let { user } = getSession(schema);
    if (!user) {
      return new Response(403, {}, { errors: [{ detail: 'must be logged in to perform that action' }] });
    }

    let { name } = request.params;
    let crate = schema.crates.findBy({ name });
    if (!crate) {
      return new Response(404, {}, { errors: [{ detail: 'Not Found' }] });
    }

    let following = user.followedCrates.includes(crate);

    return { following };
  });

  server.put('/api/v1/crates/:name/follow', (schema, request) => {
    let { user } = getSession(schema);
    if (!user) {
      return new Response(403, {}, { errors: [{ detail: 'must be logged in to perform that action' }] });
    }

    let { name } = request.params;
    let crate = schema.crates.findBy({ name });
    if (!crate) {
      return new Response(404, {}, { errors: [{ detail: 'Not Found' }] });
    }

    user.followedCrates.add(crate);
    user.save();

    return { ok: true };
  });

  server.delete('/api/v1/crates/:name/follow', (schema, request) => {
    let { user } = getSession(schema);
    if (!user) {
      return new Response(403, {}, { errors: [{ detail: 'must be logged in to perform that action' }] });
    }

    let { name } = request.params;
    let crate = schema.crates.findBy({ name });
    if (!crate) {
      return new Response(404, {}, { errors: [{ detail: 'Not Found' }] });
    }

    user.followedCrates.remove(crate);
    user.save();

    return { ok: true };
  });

  server.get('/api/v1/crates/:name/versions', (schema, request) => {
    let { name } = request.params;
    let crate = schema.crates.findBy({ name });
    if (!crate) return notFound();

    return crate.versions.sort((a, b) => compareIsoDates(b.created_at, a.created_at));
  });

  server.get('/api/v1/crates/:name/:version_num/authors', (schema, request) => {
    let { name } = request.params;
    let crate = schema.crates.findBy({ name });
    if (!crate) return notFound();

    let num = request.params.version_num;
    let version = schema.versions.findBy({ crateId: crate.id, num });
    if (!version) return { errors: [{ detail: `crate \`${crate.name}\` does not have a version \`${num}\`` }] };

    return { meta: { names: [] }, users: [] };
  });

  server.get('/api/v1/crates/:name/:version_num/dependencies', (schema, request) => {
    let { name } = request.params;
    let crate = schema.crates.findBy({ name });
    if (!crate) return notFound();

    let num = request.params.version_num;
    let version = schema.versions.findBy({ crateId: crate.id, num });
    if (!version) return { errors: [{ detail: `crate \`${crate.name}\` does not have a version \`${num}\`` }] };

    return schema.dependencies.where({ versionId: version.id });
  });

  server.get('/api/v1/crates/:name/:version_num/downloads', function (schema, request) {
    let { name } = request.params;
    let crate = schema.crates.findBy({ name });
    if (!crate) return notFound();

    let versionNum = request.params.version_num;
    let version = schema.versions.findBy({ crateId: crate.id, num: versionNum });
    if (!version) return { errors: [{ detail: `crate \`${crate.name}\` does not have a version \`${versionNum}\`` }] };

    return schema.versionDownloads.where({ versionId: version.id });
  });

  server.get('/api/v1/crates/:name/owner_user', function (schema, request) {
    let { name } = request.params;
    let crate = schema.crates.findBy({ name });
    if (!crate) return notFound();

    let ownerships = schema.crateOwnerships.where({ crateId: crate.id }).filter(it => it.userId).models;

    return {
      users: ownerships.map(it => {
        let json = this.serialize(it.user, 'user').user;
        json.kind = 'user';
        return json;
      }),
    };
  });

  server.get('/api/v1/crates/:name/owner_team', function (schema, request) {
    let { name } = request.params;
    let crate = schema.crates.findBy({ name });
    if (!crate) return notFound();

    let ownerships = schema.crateOwnerships.where({ crateId: crate.id }).filter(it => it.teamId).models;

    return {
      teams: ownerships.map(it => {
        let json = this.serialize(it.team, 'team').team;
        json.kind = 'team';
        return json;
      }),
    };
  });

  server.get('/api/v1/crates/:name/reverse_dependencies', function (schema, request) {
    let { name } = request.params;
    let crate = schema.crates.findBy({ name });
    if (!crate) return notFound();

    let { start, end } = pageParams(request);

    let allDependencies = schema.dependencies.where({ crateId: crate.id });
    let dependencies = allDependencies.slice(start, end);
    let total = allDependencies.length;

    let versions = schema.versions.find(dependencies.models.map(it => it.versionId));

    return {
      ...this.serialize(dependencies),
      ...this.serialize(versions),
      meta: { total },
    };
  });

  server.get('/api/v1/crates/:name/downloads', function (schema, request) {
    let { name } = request.params;
    let crate = schema.crates.findBy({ name });
    if (!crate) return notFound();

    let versionDownloads = schema.versionDownloads.all().filter(it => it.version.crateId === crate.id);

    return { ...this.serialize(versionDownloads), meta: { extra_downloads: crate._extra_downloads } };
  });

  server.put('/api/v1/crates/:name/owners', (schema, request) => {
    let { user } = getSession(schema);
    if (!user) {
      return new Response(403, {}, { errors: [{ detail: 'must be logged in to perform that action' }] });
    }

    let { name } = request.params;
    let crate = schema.crates.findBy({ name });

    if (!crate) {
      return notFound();
    }

    const body = JSON.parse(request.requestBody);

    let users = [];
    let teams = [];
    let msgs = [];
    for (let login of body.owners) {
      if (login.includes(':')) {
        let team = schema.teams.findBy({ login });
        if (!team) {
          return new Response(404, {}, { errors: [{ detail: `could not find team with login \`${login}\`` }] });
        }

        teams.push(team);
        msgs.push(`team ${login} has been added as an owner of crate ${crate.name}`);
      } else {
        let user = schema.users.findBy({ login });
        if (!user) {
          return new Response(404, {}, { errors: [{ detail: `could not find user with login \`${login}\`` }] });
        }

        users.push(user);
        msgs.push(`user ${login} has been invited to be an owner of crate ${crate.name}`);
      }
    }

    for (let team of teams) {
      schema.crateOwnerships.create({ crate, team });
    }

    for (let invitee of users) {
      schema.crateOwnerInvitations.create({ crate, inviter: user, invitee });
    }

    let msg = msgs.join(',');
    return { ok: true, msg };
  });

  server.delete('/api/v1/crates/:name/owners', (schema, request) => {
    let { user } = getSession(schema);
    if (!user) {
      return new Response(403, {}, { errors: [{ detail: 'must be logged in to perform that action' }] });
    }

    let { name } = request.params;
    let crate = schema.crates.findBy({ name });

    if (!crate) {
      return notFound();
    }

    const body = JSON.parse(request.requestBody);
    const [ownerId] = body.owners;
    const owner = schema.users.findBy({ login: ownerId }) || schema.teams.findBy({ login: ownerId });

    if (!owner) {
      return notFound();
    }

    return { ok: true, msg: 'owners successfully removed' };
  });

  server.patch('/api/v1/crates/:name/:version', function (schema, request) {
    let { user } = getSession(schema);
    if (!user) {
      return new Response(403, {}, { errors: [{ detail: 'must be logged in to perform that action' }] });
    }

    const { name, version: versionNum } = request.params;
    const crate = schema.crates.findBy({ name });
    if (!crate) {
      return notFound();
    }

    const version = schema.versions.findBy({ crateId: crate.id, num: versionNum });
    if (!version) {
      return notFound();
    }

    const body = JSON.parse(request.requestBody);
    version.update({
      yanked: body.version.yanked,
      yank_message: body.version.yanked ? body.version.yank_message || null : null,
    });

    return this.serialize(version);
  });

  server.delete('/api/v1/crates/:name/:version/yank', (schema, request) => {
    let { user } = getSession(schema);
    if (!user) {
      return new Response(403, {}, { errors: [{ detail: 'must be logged in to perform that action' }] });
    }

    const { name, version: versionNum } = request.params;
    const crate = schema.crates.findBy({ name });
    if (!crate) {
      return notFound();
    }

    const version = schema.versions.findBy({ crateId: crate.id, num: versionNum });
    if (!version) {
      return notFound();
    }

    version.update({ yanked: true });

    return { ok: true };
  });

  server.put('/api/v1/crates/:name/:version/unyank', (schema, request) => {
    let { user } = getSession(schema);
    if (!user) {
      return new Response(403, {}, { errors: [{ detail: 'must be logged in to perform that action' }] });
    }

    const { name, version: versionNum } = request.params;
    const crate = schema.crates.findBy({ name });
    if (!crate) {
      return notFound();
    }

    const version = schema.versions.findBy({ crateId: crate.id, num: versionNum });
    if (!version) {
      return notFound();
    }

    version.update({ yanked: false, yank_message: null });

    return { ok: true };
  });

  server.get('/api/v1/crates/:name/:version/readme', (schema, request) => {
    const { name, version: versionNum } = request.params;
    const crate = schema.crates.findBy({ name });
    if (!crate) {
      return new Response(403, { 'Content-Type': 'text/html' }, '');
    }

    const version = schema.versions.findBy({ crateId: crate.id, num: versionNum });
    if (!version || !version.readme) {
      return new Response(403, { 'Content-Type': 'text/html' }, '');
    }

    return new Response(200, { 'Content-Type': 'text/html' }, version.readme);
  });
}
