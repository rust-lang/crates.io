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
      ? versions.sort((a, b) => b.semver.compare(a.semver))
      : versions.sort((a, b) => b.created_at - a.created_at);
  }
}
