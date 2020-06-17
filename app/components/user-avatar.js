import Component from '@ember/component';
import { computed } from '@ember/object';
import { readOnly } from '@ember/object/computed';

export default class UserAvatar extends Component {
  size = 'small';
  user = null;

  tagName = '';

  @computed('size')
  get width() {
    if (this.size === 'small') {
      return 22;
    } else if (this.size === 'medium-small') {
      return 32;
    } else {
      return 85; // medium
    }
  }

  @readOnly('width') height;

  @computed('user')
  get alt() {
    return `${this.get('user.name')} (${this.get('user.login')})`;
  }

  @computed('size', 'user')
  get src() {
    return `${this.get('user.avatar')}&s=${this.width * 2}`;
  }
}
