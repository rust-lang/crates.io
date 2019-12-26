import { Factory, trait } from 'ember-cli-mirage';
import faker from 'faker';

export default Factory.extend({
  email_verified: false,
  email_verification_sent: true,
  name: () => faker.name.findName(),
  login: () => faker.internet.userName(),
  avatar: () => faker.image.imageUrl(),
  url: () => faker.internet.url(),
  kind: () => faker.lorem.words(),
  has_tokens: false,

  withVerifiedEmail: trait({
    email_verified: true,
  }),

  withTokens: trait({
    has_tokens: true,
  }),
});
