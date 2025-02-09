import { action } from '@ember/object';
import { inject as service } from '@ember/service';
import { htmlSafe } from '@ember/template';
import Component from '@glimmer/component';
import { tracked } from '@glimmer/tracking';

import styles from './row.module.css';

export default class VersionRow extends Component {
  @service session;

  @tracked focused = false;
  @tracked featuresOpen = false;

  get releaseTrackTitle() {
    let { version } = this.args;
    if (version.yanked) {
      return htmlSafe(`This version was <span class="${styles['rt-yanked']}">yanked</span>`);
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
      modifiers.push('prerelease');
    }
    if (version.isHighestOfReleaseTrack) {
      modifiers.push('latest');
    }

    let title = `Release Track: ${releaseTrack}`;
    if (modifiers.length !== 0) {
      let formattedModifiers = modifiers
        .map(modifier => {
          let klass = styles[`rt-${modifier}`];
          return klass ? `<span class='${klass}'>${modifier}</span>` : modifier;
        })
        .join(', ');

      title += ` (${formattedModifiers})`;
    }
    return htmlSafe(title);
  }

  get isOwner() {
    let userId = this.session.currentUser?.id;
    return this.args.version.crate.hasOwnerUser(userId);
  }

  @action setFocused(value) {
    this.focused = value;
  }

  @action toggleDropdown() {
    this.featuresOpen = !this.featuresOpen;
  }
}
