import compareSemver from 'semver/functions/compare-loose.js';

import { db } from '../../index.js';
import { serializeVersion } from '../../serializers/version.js';
import { notFoundError } from '../../utils/handlers.js';
import { http } from '../../utils/openapi-http.js';
import { calculateReleaseTracks } from '../../utils/release-tracks.js';

export default http.get('/api/v1/crates/{name}/versions', ({ request, params, response }) => {
  let { name } = params;
  let crate = db.crate.findFirst(q => q.where({ name }));
  if (!crate) return response('4XX').json(notFoundError(), { status: 404 });

  let versions = db.version.findMany(q => q.where(version => version.crate.id === crate.id));

  let url = new URL(request.url);
  let nums = url.searchParams.getAll('nums[]');
  if (nums.length !== 0) {
    versions = versions.filter(v => nums.includes(v.num));
  }

  let sort = url.searchParams.get('sort');
  versions =
    sort == 'date'
      ? versions.toSorted((a, b) => b.id - a.id)
      : versions.toSorted((a, b) => compareSemver(b.num, a.num));

  let total = versions.length;

  let include = url.searchParams.get('include') ?? '';
  let includes = include ? include.split(',') : [];
  let releaseTracks = includes.includes('release_tracks') ? calculateReleaseTracks(versions) : undefined;

  // seek pagination
  // A simplified seek encoding is applied here for testing purposes only. It should be opaque in
  // real-world scenarios.
  let next_seek: string | null = null;
  let nextPage: string | null = null;
  let per_page = url.searchParams.get('per_page');
  if (per_page != null) {
    let seek = url.searchParams.get('seek');
    if (seek != null) {
      let start = versions.findIndex(it => it.num === seek);
      versions = versions.slice(start + 1);
    }
    versions = versions.slice(0, parseInt(per_page));

    if (versions.length === parseInt(per_page)) {
      next_seek = versions.at(-1)!.num;
    }
  }
  if (next_seek) {
    let next_params = new URLSearchParams(url.searchParams);
    next_params.set('seek', next_seek);
    nextPage = `?${next_params}`;
  }

  let serializedVersions = versions.map(v => serializeVersion(v));
  return response(200).json({
    versions: serializedVersions,
    meta: {
      total,
      next_page: nextPage,
      ...(releaseTracks && { release_tracks: releaseTracks }),
    },
  });
});
