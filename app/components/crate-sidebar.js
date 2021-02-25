import { computed } from '@ember/object';
import { gt, readOnly } from '@ember/object/computed';
import Component from '@glimmer/component';

const NUM_VERSIONS = 5;

export default class DownloadGraph extends Component {
  @readOnly('args.crate.versions') sortedVersions;

  @computed('sortedVersions')
  get smallSortedVersions() {
    return this.sortedVersions.slice(0, NUM_VERSIONS);
  }

  @gt('sortedVersions.length', NUM_VERSIONS) hasMoreVersions;

  get tomlSnippet() {
    return `${this.args.crate.name} = "${this.args.version.num}"`;
  }
}
