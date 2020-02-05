import { Factory, trait } from 'ember-cli-mirage';
import faker from 'faker';

export default Factory.extend({
  email_verified: false,
  email_verification_sent: true,
  name() {
    return faker.name.findName();
  },
  login() {
    return faker.internet.userName();
  },
  avatar() {
    return faker.image.imageUrl();
  },
  url() {
    return faker.internet.url();
  },
  kind: 'user',
  has_tokens: false,

  withVerifiedEmail: trait({
    email_verified: true,
  }),

  withTokens: trait({
    has_tokens: true,
  }),
});
