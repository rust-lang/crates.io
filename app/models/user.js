import Model, { attr } from '@ember-data/model';

import { memberAction } from 'ember-api-actions';

export default Model.extend({
  email: attr('string'),
  email_verified: attr('boolean'),
  email_verification_sent: attr('boolean'),
  name: attr('string'),
  login: attr('string'),
  avatar: attr('string'),
  url: attr('string'),
  kind: attr('string'),

  stats: memberAction({ type: 'GET', path: 'stats' }),
});
