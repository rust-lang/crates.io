import Controller from '@ember/controller';
import { action } from '@ember/object';
import { inject as service } from '@ember/service';
import { htmlSafe } from '@ember/template';
import { tracked } from '@glimmer/tracking';

import { task } from 'ember-concurrency';
import { TrackedArray } from 'tracked-built-ins';

import { scopeDescription } from '../../../utils/token-scopes';

export default class NewTokenController extends Controller {
  @service notifications;
  @service sentry;
  @service store;
  @service router;

  @tracked name;
  @tracked nameInvalid;
  @tracked scopes;
  @tracked scopesInvalid;
  @tracked crateScopes;

  ENDPOINT_SCOPES = ['change-owners', 'publish-new', 'publish-update', 'yank'];

  scopeDescription = scopeDescription;

  constructor() {
    super(...arguments);
    this.reset();
  }

  @action isScopeSelected(id) {
    return this.scopes.includes(id);
  }

  saveTokenTask = task(async () => {
    if (!this.validate()) return;
    let { name, scopes } = this;

    let crateScopes = this.crateScopes.map(it => it.pattern);
    if (crateScopes.length === 0) {
      crateScopes = null;
    }

    let token = this.store.createRecord('api-token', {
      name,
      endpoint_scopes: scopes,
      crate_scopes: crateScopes,
    });

    try {
      // Save the new API token on the backend
      await token.save();
      // Reset the form
      this.reset();
      // Navigate to the API token list
      this.router.transitionTo('settings.tokens.index');
    } catch (error) {
      // Notify the user
      this.notifications.error('An error has occurred while generating your API token. Please try again later!');
      // Notify the crates.io team
      this.sentry.captureException(error);
      // Notify the developer
      console.error(error);
    }
  });

  reset() {
    this.name = '';
    this.nameInvalid = false;
    this.scopes = [];
    this.scopesInvalid = false;
    this.crateScopes = TrackedArray.of();
  }

  validate() {
    this.nameInvalid = !this.name;
    this.scopesInvalid = this.scopes.length === 0;
    let crateScopesValid = this.crateScopes.map(pattern => pattern.validate(false)).every(Boolean);

    return !this.nameInvalid && !this.scopesInvalid && crateScopesValid;
  }

  @action resetNameValidation() {
    this.nameInvalid = false;
  }

  @action toggleScope(id) {
    this.scopes = this.scopes.includes(id) ? this.scopes.filter(it => it !== id) : [...this.scopes, id];
    this.scopesInvalid = false;
  }

  @action addCratePattern() {
    this.crateScopes.push(new CratePattern(''));
  }

  @action removeCrateScope(index) {
    this.crateScopes.splice(index, 1);
  }
}

class CratePattern {
  @tracked pattern;
  @tracked showAsInvalid = false;

  constructor(pattern) {
    this.pattern = pattern;
  }

  get isValid() {
    return isValidPattern(this.pattern);
  }

  get hasWildcard() {
    return this.pattern.endsWith('*');
  }

  get description() {
    if (!this.pattern) {
      return 'Please enter a crate name pattern';
    } else if (this.pattern === '*') {
      return 'Matches all crates on crates.io';
    } else if (!this.isValid) {
      return 'Invalid crate name pattern';
    } else if (this.hasWildcard) {
      return htmlSafe(`Matches all crates starting with <strong>${this.pattern.slice(0, -1)}</strong>`);
    } else {
      return htmlSafe(`Matches only the <strong>${this.pattern}</strong> crate`);
    }
  }

  @action resetValidation() {
    this.showAsInvalid = false;
  }

  @action validate(ignoreEmpty = true) {
    let valid = this.isValid || (ignoreEmpty && this.pattern === '');
    this.showAsInvalid = !valid;
    return valid;
  }
}

function isValidIdent(pattern) {
  return (
    [...pattern].every(c => isAsciiAlphanumeric(c) || c === '_' || c === '-') &&
    pattern[0] !== '_' &&
    pattern[0] !== '-'
  );
}

function isValidPattern(pattern) {
  if (!pattern) return false;
  if (pattern === '*') return true;

  if (pattern.endsWith('*')) {
    pattern = pattern.slice(0, -1);
  }

  return isValidIdent(pattern);
}

function isAsciiAlphanumeric(c) {
  return (c >= '0' && c <= '9') || (c >= 'A' && c <= 'Z') || (c >= 'a' && c <= 'z');
}
