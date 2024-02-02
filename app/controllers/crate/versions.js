import Controller from '@ember/controller';
import { tracked } from '@glimmer/tracking';

export default class SearchController extends Controller {
  queryParams = ['sort'];

  @tracked sort;

  get currentSortBy() {
    return this.sort === 'semver' ? 'SemVer' : 'Date';
  }

  get sortedVersions() {
    let { versionIdsBySemver, versionIdsByDate, versionsObj: versions } = this.model;

    return (this.sort === 'semver' ? versionIdsBySemver : versionIdsByDate).map(id => versions[id]);
  }
}
