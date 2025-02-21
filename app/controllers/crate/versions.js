import Controller from '@ember/controller';
import { tracked } from '@glimmer/tracking';

export default class SearchController extends Controller {
  queryParams = ['sort'];

  @tracked sort;

  /** @type {import("../../models/crate").default} */
  @tracked crate;

  get currentSortBy() {
    return this.sort === 'semver' ? 'SemVer' : 'Date';
  }

  get sortedVersions() {
    let { versionIdsBySemver, versionIdsByDate, versionsObj: versions } = this.crate;

    return (this.sort === 'semver' ? versionIdsBySemver : versionIdsByDate).map(id => versions.get(id));
  }
}
