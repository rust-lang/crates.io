import { action } from '@ember/object';
import { inject as service } from '@ember/service';
import Component from '@glimmer/component';

import { task } from 'ember-concurrency';

import { patternDescription, scopeDescription } from '../../utils/token-scopes';

export default class ApiTokens extends Component {
  @service store;
  @service notifications;
  @service router;

  scopeDescription = scopeDescription;
  patternDescription = patternDescription;

  get sortedTokens() {
    return this.args.tokens
      .filter(t => !t.isNew)
      .sort((a, b) => {
        // Expired tokens are always shown after active ones.
        if (a.isExpired && !b.isExpired) {
          return 1;
        } else if (b.isExpired && !a.isExpired) {
          return -1;
        }

        // Otherwise, sort normally based on creation time.
        return a.created_at < b.created_at ? 1 : -1;
      });
  }

  listToParts(list) {
    // We hardcode `en-US` here because the rest of the interface text is also currently displayed only in English.
    return new Intl.ListFormat('en-US').formatToParts(list);
  }

  @action startNewToken() {
    this.router.transitionTo('settings.tokens.new');
  }

  revokeTokenTask = task(async token => {
    try {
      await token.destroyRecord();

      let index = this.args.tokens.indexOf(token);
      if (index !== -1) {
        this.args.tokens.splice(index, 1);
      }
    } catch (error) {
      let msg =
        error.errors && error.errors[0] && error.errors[0].detail
          ? `An error occurred while revoking this token, ${error.errors[0].detail}`
          : 'An unknown error occurred while revoking this token';

      this.notifications.error(msg);
    }
  });
}
