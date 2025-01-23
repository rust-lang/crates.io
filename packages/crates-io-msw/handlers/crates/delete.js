import { http, HttpResponse } from 'msw';

import { db } from '../../index.js';
import { getSession } from '../../utils/session.js';

export default http.delete('/api/v1/crates/:name', async ({ params }) => {
  let { user } = getSession();
  if (!user) {
    return HttpResponse.json({ errors: [{ detail: 'must be logged in to perform that action' }] }, { status: 403 });
  }

  let crate = db.crate.findFirst({ where: { name: { equals: params.name } } });
  if (!crate) {
    return HttpResponse.json({ errors: [{ detail: `crate \`${params.name}\` does not exist` }] }, { status: 404 });
  }

  db.crate.delete({ where: { id: crate.id } });

  return new HttpResponse(null, { status: 204 });
});
