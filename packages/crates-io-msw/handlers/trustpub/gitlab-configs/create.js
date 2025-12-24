import { http, HttpResponse } from 'msw';

import { db } from '../../../index.js';
import { serializeGitLabConfig } from '../../../serializers/trustpub/gitlab-config.js';
import { notFound } from '../../../utils/handlers.js';
import { getSession } from '../../../utils/session.js';

export default http.post('/api/v1/trusted_publishing/gitlab_configs', async ({ request }) => {
  let { user } = getSession();
  if (!user) {
    return HttpResponse.json({ errors: [{ detail: 'must be logged in to perform that action' }] }, { status: 403 });
  }

  let body = await request.json();

  let { gitlab_config } = body;
  if (!gitlab_config) {
    return HttpResponse.json({ errors: [{ detail: 'invalid request body' }] }, { status: 400 });
  }

  let { crate: crateName, namespace, project, workflow_filepath, environment } = gitlab_config;
  if (!crateName || !namespace || !project || !workflow_filepath) {
    return HttpResponse.json({ errors: [{ detail: 'missing required fields' }] }, { status: 400 });
  }

  let crate = db.crate.findFirst(q => q.where({ name: crateName }));
  if (!crate) return notFound();

  // Check if the user is an owner of the crate
  let isOwner = db.crateOwnership.findFirst(q =>
    q.where(ownership => ownership.crate.id === crate.id && ownership.user?.id === user.id),
  );
  if (!isOwner) {
    return HttpResponse.json({ errors: [{ detail: 'You are not an owner of this crate' }] }, { status: 400 });
  }

  // Check if the user has a verified email
  let hasVerifiedEmail = user.emailVerified;
  if (!hasVerifiedEmail) {
    let detail = 'You must verify your email address to create a Trusted Publishing config';
    return HttpResponse.json({ errors: [{ detail }] }, { status: 403 });
  }

  // Create a new GitLab config
  let config = await db.trustpubGitlabConfig.create({
    crate,
    namespace,
    namespace_id: null,
    project,
    workflow_filepath,
    environment: environment ?? null,
    created_at: new Date().toISOString(),
  });

  return HttpResponse.json({
    gitlab_config: serializeGitLabConfig(config),
  });
});
