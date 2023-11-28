import Controller from '@ember/controller';
import { tracked } from '@glimmer/tracking';

export default class SearchController extends Controller {
  queryParams = ['sort'];

  @tracked sort;

  get currentSortBy() {
    return this.sort === 'semver' ? 'SemVer' : 'Date';
  }

  get sortedVersions() {
    let versions = this.model.versions.toArray();

    return this.sort === 'semver'
      ? versions.sort(compareBySemver)
      : versions.sort((a, b) => b.created_at - a.created_at);
  }
}

function compareBySemver(a, b) {
  let aSemver = a.semver;
  let bSemver = b.semver;

  if (aSemver === bSemver) {
    return b.created_at - a.created_at;
  } else if (aSemver === null) {
    return 1;
  } else if (bSemver === null) {
    return -1;
  } else {
    return bSemver.compare(aSemver);
  }
}
