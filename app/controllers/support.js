import Controller from '@ember/controller';
import { tracked } from '@glimmer/tracking';

const SUPPORTS = [
  {
    inquire: 'crate-violation',
    label: 'Report a crate that violates policies',
  },
];

const VALID_INQUIRE = new Set(SUPPORTS.map(s => s.inquire));

export default class SupportController extends Controller {
  queryParams = ['inquire', 'crate'];

  @tracked inquire;
  @tracked crate;

  supports = SUPPORTS;

  get supported() {
    return VALID_INQUIRE.has(this.inquire);
  }
}
