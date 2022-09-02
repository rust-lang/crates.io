import Controller from '@ember/controller';
import { tracked } from '@glimmer/tracking';

import { reads } from 'macro-decorators';

import { pagination } from '../../utils/pagination';

export default class PendingInvitesController extends Controller {
  queryParams = ['page'];
  @tracked page = '1';

  @reads('model.meta.total') totalItems;
  @pagination() pagination;
}
