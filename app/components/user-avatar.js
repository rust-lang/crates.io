import Component from '@ember/component';
import { computed } from '@ember/object';
import { readOnly } from '@ember/object/computed';

export default class UserAvatar extends Component {
  tagName = '';

  @computed('size')
  get width() {
    if (this.size === 'medium') {
      return 85;
    } else if (this.size === 'medium-small') {
      return 32;
    } else {
      return 22; // small
    }
  }

  @readOnly('width') height;

  @computed('user')
  get alt() {
    return `${this.user.name} (${this.user.login})`;
  }

  @computed('size', 'user')
  get src() {
    return `${this.user.avatar}&s=${this.width * 2}`;
  }
}
