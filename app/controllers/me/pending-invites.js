import Controller from '@ember/controller';

import { reads } from 'macro-decorators';

import { pagination } from '../../utils/seek-pagination';

export default class PendingInvitesController extends Controller {
  @reads('model.meta.next_page') nextPage;
  @pagination() pagination;
}
