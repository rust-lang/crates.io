import { Factory } from 'ember-cli-mirage';
import { dasherize } from '@ember/string';

export default Factory.extend({
  name: i => `User ${i + 1}`,

  login() {
    return dasherize(this.name);
  },

  url() {
    return `https://github.com/${this.login}`;
  },

  avatar: 'https://avatars1.githubusercontent.com/u/14631425?v=4',
});
