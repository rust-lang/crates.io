import Controller from '@ember/controller';
import { tracked } from '@glimmer/tracking';

import { reads } from 'macro-decorators';

import { pagination } from '../../utils/seek';

export default class PendingInvitesController extends Controller {
  queryParams = ['seek'];
  @tracked seek = 'WzEsIDFd';

  @reads('model.meta.total') totalItems;
  @pagination() pagination;
}
