import { action } from '@ember/object';
import { notEmpty, filterBy, sort } from '@ember/object/computed';
import { inject as service } from '@ember/service';
import Component from '@glimmer/component';

export default class ApiTokens extends Component {
  @service store;

  tokenSort = ['created_at:desc'];
  @sort('args.tokens', 'tokenSort') sortedTokens;
  @filterBy('args.tokens', 'isNew', true) newTokens;
  @notEmpty('newTokens') disableCreate;

  @action startNewToken() {
    this.store.createRecord('api-token', {
      created_at: new Date(Date.now() + 2000),
    });
  }
}
