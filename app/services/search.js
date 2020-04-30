import { oneWay } from '@ember/object/computed';
import Service from '@ember/service';

export default class SearchService extends Service {
  q = null;

  @oneWay('q') inputValue;
}
