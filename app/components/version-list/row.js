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
      return 'This version was <mark style="color: hsl(0, 84%, 32%)">yanked</mark>';
    }
    if (version.invalidSemver) {
      return `Failed to parse version ${version.num}`;
    }
    if (version.isFirst) {
      return 'This is the first version that was released';
    }

    let { releaseTrack } = version;

    let modifiers = [];
    if (version.isPrerelease) {
      modifiers.push('<mark style="color: hsl(39, 71%, 45%)">prerelease</mark>');
    }
    if (version.isHighestOfReleaseTrack) {
      modifiers.push('<mark style="color: hsl(136, 67%, 38%)">latest</mark>');
    }

    let title = `Release Track: ${releaseTrack}`;
    if (modifiers.length !== 0) {
      title += ` (${modifiers.join(', ')})`;
    }
    return title;
  }

  get isOwner() {
    return this.args.version.crate?.owner_user?.findBy('id', this.session.currentUser?.id);
  }

  @action setFocused(value) {
    this.focused = value;
  }
}
