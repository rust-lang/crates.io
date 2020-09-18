import Model, { attr } from '@ember-data/model';

import { memberAction } from 'ember-api-actions';

export default class User extends Model {
  @attr email;
  @attr email_verified;
  @attr email_verification_sent;
  @attr name;
  @attr login;
  @attr avatar;
  @attr url;
  @attr kind;

  stats = memberAction({ type: 'GET', path: 'stats' });
}
