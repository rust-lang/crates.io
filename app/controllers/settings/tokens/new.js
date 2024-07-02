import Controller from '@ember/controller';
import { action } from '@ember/object';
import { inject as service } from '@ember/service';
import { tracked } from '@glimmer/tracking';

import { task } from 'ember-concurrency';
import { TrackedArray } from 'tracked-built-ins';

import { patternDescription, scopeDescription } from '../../../utils/token-scopes';

export default class NewTokenController extends Controller {
  @service notifications;
  @service sentry;
  @service store;
  @service router;

  @tracked name;
  @tracked nameInvalid;
  @tracked expirySelection;
  @tracked expiryDateInput;
  @tracked expiryDateInvalid;
  @tracked scopes;
  @tracked scopesInvalid;
  @tracked crateScopes;

  ENDPOINT_SCOPES = ['change-owners', 'publish-new', 'publish-update', 'yank'];

  scopeDescription = scopeDescription;

  constructor() {
    super(...arguments);
    this.reset();
  }

  get today() {
    return new Date().toISOString().slice(0, 10);
  }

  get expiryDate() {
    if (this.expirySelection === 'none') return null;
    if (this.expirySelection === 'custom') {
      if (!this.expiryDateInput) return null;

      let now = new Date();
      let timeSuffix = now.toISOString().slice(10);
      return new Date(this.expiryDateInput + timeSuffix);
    }

    let date = new Date();
    date.setDate(date.getDate() + Number(this.expirySelection));
    return date;
  }

  get expiryDescription() {
    return this.expirySelection === 'none'
      ? 'The token will never expire'
      : `The token will expire on ${this.expiryDate.toLocaleDateString(undefined, { dateStyle: 'long' })}`;
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
      expired_at: this.expiryDate,
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
    this.expirySelection = 'none';
    this.expiryDateInput = null;
    this.expiryDateInvalid = false;
    this.scopes = [];
    this.scopesInvalid = false;
    this.crateScopes = TrackedArray.of();
  }

  validate() {
    this.nameInvalid = !this.name;
    this.expiryDateInvalid = this.expirySelection === 'custom' && !this.expiryDateInput;
    this.scopesInvalid = this.scopes.length === 0;
    let crateScopesValid = this.crateScopes.map(pattern => pattern.validate(false)).every(Boolean);

    return !this.nameInvalid && !this.expiryDateInvalid && !this.scopesInvalid && crateScopesValid;
  }

  @action resetNameValidation() {
    this.nameInvalid = false;
  }

  @action updateExpirySelection(event) {
    this.expiryDateInput = this.expiryDate?.toISOString().slice(0, 10);
    this.expirySelection = event.target.value;
  }

  @action resetExpiryDateValidation() {
    this.expiryDateInvalid = false;
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

export class CratePattern {
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
    } else if (this.isValid) {
      return patternDescription(this.pattern);
    } else {
      return 'Invalid crate name pattern';
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
