import { http, HttpResponse } from 'msw';

import { db } from '../../../index.js';
import { serializeGitHubConfig } from '../../../serializers/trustpub/github-config.js';
import { notFound } from '../../../utils/handlers.js';
import { getSession } from '../../../utils/session.js';

export default http.get('/api/v1/trusted_publishing/github_configs', ({ request }) => {
  let url = new URL(request.url);

  let { user } = getSession();
  if (!user) {
    return HttpResponse.json({ errors: [{ detail: 'must be logged in to perform that action' }] }, { status: 403 });
  }

  let crateName = url.searchParams.get('crate');
  if (!crateName) {
    return HttpResponse.json({ errors: [{ detail: 'missing or invalid filter' }] }, { status: 400 });
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

  let configs = db.trustpubGithubConfig.findMany(q => q.where(config => config.crate.id === crate.id));

  return HttpResponse.json({
    github_configs: configs.map(config => serializeGitHubConfig(config)),
  });
});
