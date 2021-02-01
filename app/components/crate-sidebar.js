import { computed } from '@ember/object';
import { gt, readOnly } from '@ember/object/computed';
import { inject as service } from '@ember/service';
import Component from '@glimmer/component';

const NUM_VERSIONS = 5;

export default class DownloadGraph extends Component {
  @service session;

  @computed('args.crate.owner_user', 'session.currentUser.id')
  get isOwner() {
    return this.args.crate.owner_user.findBy('id', this.session.currentUser?.id);
  }

  @readOnly('args.crate.versions') sortedVersions;

  @computed('sortedVersions')
  get smallSortedVersions() {
    return this.sortedVersions.slice(0, NUM_VERSIONS);
  }

  @gt('sortedVersions.length', NUM_VERSIONS) hasMoreVersions;
}
