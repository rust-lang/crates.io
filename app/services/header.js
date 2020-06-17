import Service from '@ember/service';
import { tracked } from '@glimmer/tracking';

export default class SearchService extends Service {
  // the value of the search input fields in the header
  @tracked searchValue = null;
}
