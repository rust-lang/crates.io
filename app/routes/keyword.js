import Route from '@ember/routing/route';

export default class KeywordRoute extends Route {
  model({ keyword_id }) {
    return keyword_id;
  }
}
