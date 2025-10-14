import { http, HttpResponse } from 'msw';

import { db } from '../../../index.js';
import { notFound } from '../../../utils/handlers.js';
import { getSession } from '../../../utils/session.js';

export default http.delete('/api/v1/trusted_publishing/github_configs/:id', ({ params }) => {
  let { user } = getSession();
  if (!user) {
    return HttpResponse.json({ errors: [{ detail: 'must be logged in to perform that action' }] }, { status: 403 });
  }

  let id = parseInt(params.id);
  let config = db.trustpubGithubConfig.findFirst(q => q.where({ id }));
  if (!config) return notFound();

  // Check if the user is an owner of the crate
  let isOwner = db.crateOwnership.findFirst(q =>
    q.where(ownership => ownership.crate.id === config.crate.id && ownership.user?.id === user.id),
  );
  if (!isOwner) {
    return HttpResponse.json({ errors: [{ detail: 'You are not an owner of this crate' }] }, { status: 400 });
  }

  // Delete the config
  db.trustpubGithubConfig.delete(q => q.where({ id }));

  return new HttpResponse(null, { status: 204 });
});
