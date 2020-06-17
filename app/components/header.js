import Component from '@ember/component';
import { action } from '@ember/object';
import { inject as service } from '@ember/service';

export default class Header extends Component {
  @service header;
  @service router;
  @service session;

  tagName = '';

  @action
  search(event) {
    event.preventDefault();

    this.router.transitionTo('search', {
      queryParams: {
        q: this.header.searchValue,
        page: 1,
      },
    });
  }
}
