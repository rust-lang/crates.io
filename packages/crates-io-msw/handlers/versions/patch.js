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

  let crate = db.crate.findFirst({ where: { name: { equals: params.name } } });
  if (!crate) return notFound();

  let version = db.version.findFirst({
    where: {
      crate: { id: { equals: crate.id } },
      num: { equals: params.version },
    },
  });
  if (!version) return notFound();

  let body = await request.json();

  let yanked = body.version.yanked;
  let yankMessage = body.version.yank_message;

  version = db.version.update({
    where: { id: { equals: version.id } },
    data: {
      yanked: yanked,
      yank_message: yanked ? yankMessage || null : null,
    },
  });

  return HttpResponse.json({ version: serializeVersion(version) });
});
