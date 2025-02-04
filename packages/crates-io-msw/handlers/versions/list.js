import { http, HttpResponse } from 'msw';
import compareSemver from 'semver/functions/compare-loose.js';

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

  let sort = url.searchParams.get('sort');
  versions =
    sort == 'date' ? versions.sort((a, b) => b.id - a.id) : versions.sort((a, b) => compareSemver(b.num, a.num));

  let total = versions.length;

  let include = url.searchParams.get('include') ?? '';
  let includes = include ? include.split(',') : [];
  let meta = { total, next_page: null };

  if (includes.includes('release_tracks')) {
    meta.release_tracks = calculateReleaseTracks(versions);
  }

  // seek pagination
  // A simplified seek encoding is applied here for testing purposes only. It should be opaque in
  // real-world scenarios.
  let next_seek = null;
  let per_page = url.searchParams.get('per_page');
  if (per_page != null) {
    let seek = url.searchParams.get('seek');
    if (seek != null) {
      let start = versions.findIndex(it => it.num === seek);
      versions = versions.slice(start + 1);
    }
    versions = versions.slice(0, parseInt(per_page));

    if (versions.length == per_page) {
      next_seek = versions.at(-1).num;
    }
  }
  if (next_seek) {
    let next_params = new URLSearchParams(url.searchParams);
    next_params.set('seek', next_seek);
    meta.next_page = `?${next_params}`;
  }

  let serializedVersions = versions.map(v => serializeVersion(v, { includePublishedBy: true }));
  return HttpResponse.json({ versions: serializedVersions, meta });
});
