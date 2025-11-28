import { http, HttpResponse } from 'msw';

import { db } from '../../index.js';
import { serializeVersion } from '../../serializers/version.js';
import { notFound } from '../../utils/handlers.js';
import { getSession } from '../../utils/session.js';

export default http.patch('/api/v1/crates/:name/:version', async ({ request, params }) => {
  let { user } = getSession();
  if (!user) {
    return HttpResponse.json({ errors: [{ detail: 'must be logged in to perform that action' }] }, { status: 403 });
  }

  let crate = db.crate.findFirst(q => q.where({ name: params.name }));
  if (!crate) return notFound();

  let version = db.version.findFirst(q =>
    q.where(version => version.crate.id === crate.id && version.num === params.version),
  );
  if (!version) return notFound();

  let body = await request.json();

  let yanked = body.version.yanked;
  let yankMessage = body.version.yank_message;

  version = await db.version.update(q => q.where({ id: version.id }), {
    data(version) {
      version.yanked = yanked;
      version.yank_message = yanked ? yankMessage || null : null;
    },
  });

  return HttpResponse.json({ version: serializeVersion(version) });
});
