import Component from '@glimmer/component';

export default class extends Component {
  get icon() {
    return this.args.icon ?? 'crate';
  }
}
