import { db } from '../../../index.js';
import { serializeGitHubConfig } from '../../../serializers/trustpub/github-config.js';
import { notFound } from '../../../utils/handlers.js';
import { http } from '../../../utils/openapi-http.js';
import { getSession } from '../../../utils/session.js';

export default http.post('/api/v1/trusted_publishing/github_configs', async ({ request, response }) => {
  let { user } = getSession();
  if (!user) {
    return response.untyped(
      Response.json({ errors: [{ detail: 'must be logged in to perform that action' }] }, { status: 403 }),
    );
  }

  let body = await request.json();

  let { github_config } = body;
  if (!github_config) {
    return response.untyped(Response.json({ errors: [{ detail: 'invalid request body' }] }, { status: 400 }));
  }

  let { crate: crateName, repository_owner, repository_name, workflow_filename, environment } = github_config;
  if (!crateName || !repository_owner || !repository_name || !workflow_filename) {
    return response.untyped(Response.json({ errors: [{ detail: 'missing required fields' }] }, { status: 400 }));
  }

  let crate = db.crate.findFirst(q => q.where({ name: crateName }));
  if (!crate) return response.untyped(notFound());

  // Check if the user is an owner of the crate
  let isOwner = db.crateOwnership.findFirst(q =>
    q.where(ownership => ownership.crate.id === crate.id && ownership.user?.id === user.id),
  );
  if (!isOwner) {
    return response.untyped(
      Response.json({ errors: [{ detail: 'You are not an owner of this crate' }] }, { status: 400 }),
    );
  }

  // Check if the user has a verified email
  let hasVerifiedEmail = user.emailVerified;
  if (!hasVerifiedEmail) {
    let detail = 'You must verify your email address to create a Trusted Publishing config';
    return response.untyped(Response.json({ errors: [{ detail }] }, { status: 403 }));
  }

  // Create a new GitHub config
  let config = await db.trustpubGithubConfig.create({
    crate,
    repository_owner,
    repository_name,
    workflow_filename,
    environment: environment ?? null,
    created_at: new Date().toISOString(),
  });

  return response(200).json({
    github_config: serializeGitHubConfig(config),
  });
});
