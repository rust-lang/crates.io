import { action } from '@ember/object';
import { inject as service } from '@ember/service';
import Component from '@glimmer/component';

export default class Header extends Component {
  @service header;
  @service router;

  @action updateSearchValue(event) {
    let { value } = event.target;
    this.header.searchValue = value;
  }

  @action search() {
    this.router.transitionTo('search', {
      queryParams: {
        q: this.header.searchValue,
        page: 1,
      },
    });
  }
}
