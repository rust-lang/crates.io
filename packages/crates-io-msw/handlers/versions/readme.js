import { http, HttpResponse } from 'msw';

import { db } from '../../index.js';

export default http.get('/api/v1/crates/:name/:version/readme', async ({ params }) => {
  let crate = db.crate.findFirst(q => q.where({ name: params.name }));
  if (!crate) return HttpResponse.html('', { status: 403 });

  let version = db.version.findFirst(q =>
    q.where(version => version.crate.id === crate.id && version.num === params.version),
  );
  if (!version || !version.readme) return HttpResponse.html('', { status: 403 });

  return HttpResponse.html(version.readme);
});
