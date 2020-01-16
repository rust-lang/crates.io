import { pageParams, compareStrings, withMeta, compareIsoDates, notFound } from './-utils';

export function register(server) {
  server.get('/api/v1/crates', function(schema, request) {
    const { start, end } = pageParams(request);

    let crates = schema.crates.all();

    if (request.queryParams.letter) {
      let letter = request.queryParams.letter.toLowerCase();
      crates = crates.filter(crate => crate.id[0].toLowerCase() === letter);
    }

    if (request.queryParams.q) {
      let q = request.queryParams.q.toLowerCase();
      crates = crates.filter(crate => crate.id.toLowerCase().indexOf(q) !== -1);
    }

    if (request.queryParams.user_id) {
      let userId = parseInt(request.queryParams.user_id, 10);
      crates = crates.filter(crate => (crate._owner_users || []).indexOf(userId) !== -1);
    }

    if (request.queryParams.team_id) {
      let teamId = parseInt(request.queryParams.team_id, 10);
      crates = crates.filter(crate => (crate._owner_teams || []).indexOf(teamId) !== -1);
    }

    if (request.queryParams.sort === 'alpha') {
      crates = crates.sort((a, b) => compareStrings(a.id.toLowerCase(), b.id.toLowerCase()));
    }

    return withMeta(this.serialize(crates.slice(start, end)), { total: crates.length });
  });

  server.get('/api/v1/crates/:crate_id', function(schema, request) {
    let crateId = request.params.crate_id;
    let crate = schema.crates.find(crateId);
    if (!crate) return notFound();

    return {
      ...this.serialize(crate),
      ...this.serialize(crate.categories),
      ...this.serialize(crate.keywords),
      ...this.serialize(crate.versions),
    };
  });

  server.get('/api/v1/crates/:crate_id/following', (/* schema, request */) => {
    // TODO
  });

  server.get('/api/v1/crates/:crate_id/versions', (schema, request) => {
    let crateId = request.params.crate_id;
    let crate = schema.crates.find(crateId);
    if (!crate) return notFound();

    return crate.versions.sort((a, b) => compareIsoDates(b.created_at, a.created_at));
  });

  server.get('/api/v1/crates/:crate_id/:version_num/authors', (schema, request) => {
    let crateId = request.params.crate_id;
    let crate = schema.crates.find(crateId);
    if (!crate) return notFound();

    let num = request.params.version_num;
    let version = schema.versions.findBy({ crateId, num });
    if (!version) return { errors: [{ detail: `crate \`${crateId}\` does not have a version \`${num}\`` }] };

    return { meta: { names: version._authors }, users: [] };
  });

  server.get('/api/v1/crates/:crate_id/:version_num/dependencies', (schema, request) => {
    let crateId = request.params.crate_id;
    let crate = schema.crates.find(crateId);
    if (!crate) return notFound();

    let num = request.params.version_num;
    let version = schema.versions.findBy({ crateId, num });
    if (!version) return { errors: [{ detail: `crate \`${crateId}\` does not have a version \`${num}\`` }] };

    return schema.dependencies.where({ versionId: version.id });
  });

  server.get('/api/v1/crates/:crate_id/:version_num/downloads', function(schema, request) {
    let crateId = request.params.crate_id;
    let crate = schema.crates.find(crateId);
    if (!crate) return notFound();

    let versionNum = request.params.version_num;
    let version = schema.versions.findBy({ crateId, num: versionNum });
    if (!version) return { errors: [{ detail: `crate \`${crateId}\` does not have a version \`${versionNum}\`` }] };

    return schema.versionDownloads.where({ versionId: version.id });
  });

  server.get('/api/v1/crates/:crate_id/owner_user', function(schema, request) {
    let crateId = request.params.crate_id;
    let crate = schema.crates.find(crateId);
    if (!crate) return notFound();

    let response = this.serialize(crate.userOwners);

    response.users.forEach(user => {
      user.kind = 'user';
    });

    return response;
  });

  server.get('/api/v1/crates/:crate_id/owner_team', function(schema, request) {
    let crateId = request.params.crate_id;
    let crate = schema.crates.find(crateId);
    if (!crate) return notFound();

    let response = this.serialize(crate.teamOwners);

    response.teams.forEach(team => {
      team.kind = 'team';
    });

    return response;
  });

  server.get('/api/v1/crates/:crate_id/reverse_dependencies', function(schema, request) {
    let crateId = request.params.crate_id;
    let crate = schema.crates.find(crateId);
    if (!crate) return notFound();

    let { start, end } = pageParams(request);

    let allDependencies = schema.dependencies.where({ crateId: crateId });
    let dependencies = allDependencies.slice(start, end);
    let total = allDependencies.length;

    let versions = schema.versions.find(dependencies.models.map(it => it.versionId));

    return {
      ...this.serialize(dependencies),
      ...this.serialize(versions),
      meta: { total },
    };
  });

  server.get('/api/v1/crates/:crate_id/downloads', function(schema, request) {
    let crateId = request.params.crate_id;
    let crate = schema.crates.find(crateId);
    if (!crate) return notFound();

    let versionDownloads = schema.versionDownloads.all().filter(it => it.version.crateId === crateId);

    return withMeta(this.serialize(versionDownloads), { extra_downloads: crate._extra_downloads });
  });

  server.put('/api/v1/crates/:crate_id/owners', (schema, request) => {
    const crateId = request.params.crate_id;
    const crate = schema.crates.find(crateId);

    if (!crate) {
      return notFound();
    }

    const body = JSON.parse(request.requestBody);
    const [ownerId] = body.owners;
    const user = schema.users.findBy({ login: ownerId });

    if (!user) {
      return notFound();
    }

    return { ok: true };
  });

  server.delete('/api/v1/crates/:crate_id/owners', (schema, request) => {
    const crateId = request.params.crate_id;
    const crate = schema.crates.find(crateId);

    if (!crate) {
      return notFound();
    }

    const body = JSON.parse(request.requestBody);
    const [ownerId] = body.owners;
    const user = schema.users.findBy({ login: ownerId });

    if (!user) {
      return notFound();
    }

    return {};
  });
}
