import Component from '@glimmer/component';

export default class YankButton extends Component {
  get tagName() {
    return '';
  }

  get localClass() {
    if (this.args.tan) {
      return 'tan-button';
    }

    return 'yellow-button';
  }
}
