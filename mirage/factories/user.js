import { dasherize } from '@ember/string';

import { Factory } from 'miragejs';

export default Factory.extend({
  name: i => `User ${i + 1}`,

  login() {
    return dasherize(this.name);
  },

  email() {
    return `${this.login}@crates.io`;
  },

  url() {
    return `https://github.com/${this.login}`;
  },

  avatar: 'https://avatars1.githubusercontent.com/u/14631425?v=4',

  emailVerified: null,
  emailVerificationToken: null,

  afterCreate(model) {
    if (model.emailVerified === null) {
      model.update({ emailVerified: model.email && !model.emailVerificationToken });
    }
  },
});
