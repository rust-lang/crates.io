import Model, { attr } from '@ember-data/model';

export default class Team extends Model {
  @attr email;
  @attr name;
  @attr login;
  @attr api_token;
  @attr avatar;
  @attr url;
  @attr kind;

  get org_name() {
    let login = this.login;
    let login_split = login.split(':');
    return login_split[1];
  }

  get display_name() {
    return `${this.org_name}/${this.name}`;
  }
}
