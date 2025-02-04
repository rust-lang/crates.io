import { http, HttpResponse } from 'msw';

import { db } from '../../index.js';

export default http.get('/api/v1/crates/:name/:version/readme', async ({ params }) => {
  let crate = db.crate.findFirst({ where: { name: { equals: params.name } } });
  if (!crate) return HttpResponse.html('', { status: 403 });

  let version = db.version.findFirst({
    where: {
      crate: { id: { equals: crate.id } },
      num: { equals: params.version },
    },
  });
  if (!version || !version.readme) return HttpResponse.html('', { status: 403 });

  return HttpResponse.html(version.readme);
});
