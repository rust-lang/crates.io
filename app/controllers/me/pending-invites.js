import Controller from '@ember/controller';
import { tracked } from '@glimmer/tracking';

import { reads } from 'macro-decorators';

import { pagination } from '../../utils/pagination';

export default class PendingInvitesController extends Controller {
  queryParams = ['page', 'per_page'];
  @tracked page = '1';
  @tracked per_page = 10;

  @reads('model.meta.total') totalItems;
  @pagination() pagination;
}
