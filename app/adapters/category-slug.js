import { decamelize, underscore } from '@ember/string';

import { pluralize } from 'ember-inflector';

import ApplicationAdapter from './application';

export default class CategorySlugAdapter extends ApplicationAdapter {
  pathForType(modelName) {
    let decamelized = underscore(decamelize(modelName));
    return pluralize(decamelized);
  }
}
