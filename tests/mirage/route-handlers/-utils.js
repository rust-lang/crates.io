import { Response } from 'miragejs';
import semverParse from 'semver/functions/parse';
import semverSort from 'semver/functions/rsort';

export function notFound() {
  return new Response(
    404,
    { 'Content-Type': 'application/json' },
    {
      errors: [{ detail: 'Not Found' }],
    },
  );
}

export function pageParams(request) {
  const { queryParams } = request;

  const page = parseInt(queryParams.page || '1');
  const perPage = parseInt(queryParams.per_page || '10');

  const start = (page - 1) * perPage;
  const end = start + perPage;

  return { page, perPage, start, end };
}

export function compareStrings(a, b) {
  return a < b ? -1 : a > b ? 1 : 0;
}

export function compareIsoDates(a, b) {
  let aDate = new Date(a);
  let bDate = new Date(b);
  return aDate < bDate ? -1 : aDate > bDate ? 1 : 0;
}

export function releaseTracks(versions) {
  let versionNums = versions.models.filter(it => !it.yanked).map(it => it.num);
  semverSort(versionNums, { loose: true });
  let tracks = {};
  for (let num of versionNums) {
    let semver = semverParse(num, { loose: true });
    if (!semver || semver.prerelease.length !== 0) continue;
    let name = semver.major == 0 ? `0.${semver.minor}` : `${semver.major}`;
    if (name in tracks) continue;
    tracks[name] = { highest: num };
  }
  return tracks;
}
