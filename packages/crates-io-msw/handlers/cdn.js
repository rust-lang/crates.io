import { http, HttpResponse } from 'msw';

import { db } from '../index.js';

export default [
  http.get('https://static.crates.io/readmes/:name/:filename', async ({ params }) => {
    let crate = db.crate.findFirst(q => q.where({ name: params.name }));
    if (!crate) return HttpResponse.html('', { status: 403 });

    // The expected filename is `${name}-${version}.html`. This recovers the version
    // and decodes the `%2B` applied to versions with build metadata.
    let version = decodeURIComponent(params.filename.slice(`${params.name}-`.length, -'.html'.length));

    let found = db.version.findFirst(q => q.where(v => v.crate.id === crate.id && v.num === version));
    if (!found || !found.readme) return HttpResponse.html('', { status: 403 });

    return HttpResponse.html(found.readme);
  }),
];
