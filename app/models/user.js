import Model, { attr } from '@ember-data/model';

import { memberAction } from 'ember-api-actions';

export default class User extends Model {
  @attr('string') email;
  @attr('boolean') email_verified;
  @attr('boolean') email_verification_sent;
  @attr('string') name;
  @attr('string') login;
  @attr('string') avatar;
  @attr('string') url;
  @attr('string') kind;

  stats = memberAction({ type: 'GET', path: 'stats' });
}
