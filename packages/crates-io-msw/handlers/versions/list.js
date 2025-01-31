import { http, HttpResponse } from 'msw';

import { db } from '../../index.js';
import { serializeVersion } from '../../serializers/version.js';
import { notFound } from '../../utils/handlers.js';
import { calculateReleaseTracks } from '../../utils/release-tracks.js';

export default http.get('/api/v1/crates/:name/versions', async ({ request, params }) => {
  let { name } = params;
  let crate = db.crate.findFirst({ where: { name: { equals: name } } });
  if (!crate) return notFound();

  let versions = db.version.findMany({ where: { crate: { id: { equals: crate.id } } } });

  let url = new URL(request.url);
  let nums = url.searchParams.getAll('nums[]');
  if (nums.length !== 0) {
    versions = versions.filter(v => nums.includes(v.num));
  }

  versions.sort((a, b) => b.id - a.id);
  let total = versions.length;

  let include = url.searchParams.get('include') ?? '';
  let includes = include ? include.split(',') : [];

  let serializedVersions = versions.map(v => serializeVersion(v, { includePublishedBy: true }));
  let meta = { total, next_page: null };

  if (includes.includes('release_tracks')) {
    meta.release_tracks = calculateReleaseTracks(versions);
  }

  return HttpResponse.json({ versions: serializedVersions, meta });
});
