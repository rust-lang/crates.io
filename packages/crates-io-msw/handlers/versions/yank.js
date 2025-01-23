import { http, HttpResponse } from 'msw';

import { db } from '../../index.js';
import { notFound } from '../../utils/handlers.js';
import { getSession } from '../../utils/session.js';

export default http.delete('/api/v1/crates/:name/:version/yank', async ({ params }) => {
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

  db.version.update({ where: { id: { equals: version.id } }, data: { yanked: true } });

  return HttpResponse.json({ ok: true });
});
