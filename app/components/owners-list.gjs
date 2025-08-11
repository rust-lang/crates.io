import Component from '@glimmer/component';

export default class VersionRow extends Component {
  get showDetailedList() {
    return this.args.owners.length <= 5;
  }
}
