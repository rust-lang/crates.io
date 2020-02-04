import { Factory } from 'ember-cli-mirage';
import { dasherize } from '@ember/string';

export default Factory.extend({
  avatar: 'https://avatars1.githubusercontent.com/u/14631425?v=4',
  email_verification_sent: true,
  email_verified: false,
  has_tokens: false,
  kind: "Test text",
  login: () => dasherize(this.name),
  name: i => `User ${i + 1}`,
  url: () => `https://github.com/${this.login}`,

  withVerifiedEmail: trait({
    email_verified: true,
  }),

  withTokens: trait({
    has_tokens: true,
  }),
});
