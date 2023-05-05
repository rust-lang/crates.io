import { action } from '@ember/object';
import { inject as service } from '@ember/service';
import Component from '@glimmer/component';
import { tracked } from '@glimmer/tracking';

export default class VersionRow extends Component {
  @service session;

  @tracked focused = false;

  get releaseTrackTitle() {
    let { version } = this.args;
    if (version.yanked) {
      return 'This version was yanked';
    }
    if (version.invalidSemver) {
      return `Failed to parse version ${version.num}`;
    }
    if (version.isFirst) {
      return 'This is the first version that was released';
    }

    let { releaseTrack } = version;

    return `Release Track: ${releaseTrack}`;
  }

  get displaysReleaseTrackModifiers() {
    let { version } = this.args;

    return (version.isPrerelease || version.isHighestOfReleaseTrack) && !version.yanked;
  }

  get hasAllReleaseTrackModifiers() {
    let { version } = this.args;

    return version.isPrerelease && version.isHighestOfReleaseTrack;
  }

  get isOwner() {
    return this.args.version.crate?.owner_user?.findBy('id', this.session.currentUser?.id);
  }

  @action setFocused(value) {
    this.focused = value;
  }
}
