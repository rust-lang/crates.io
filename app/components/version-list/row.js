import { action } from '@ember/object';
import { service } from '@ember/service';
import { htmlSafe } from '@ember/template';
import Component from '@glimmer/component';
import { tracked } from '@glimmer/tracking';

import styles from './row.module.css';

export default class VersionRow extends Component {
  @service session;

  @tracked focused = false;

  get releaseTrackTitle() {
    let { version } = this.args;
    if (version.yanked) {
      return htmlSafe(`This version was <span class="${styles['rt-yanked']}">yanked</span>`);
    }
    if (version.invalidSemver) {
      return `Failed to parse version ${version.num}`;
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

  get features() {
    let features = this.args.version.featureList;
    let list = features.slice(0, 15);
    let more = features.length - list.length;
    return { list, more };
  }

  @action setFocused(value) {
    this.focused = value;
  }
}
