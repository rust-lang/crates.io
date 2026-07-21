import type { SuccessBody } from '../utils/api-types.js';

import { http, HttpResponse } from 'msw';

const EXAMPLE_SHA1 = '5048d31943118c6d67359bd207d307c854e82f45';

export default [
  http.get('/api/v1/site_metadata', () => {
    return HttpResponse.json<SuccessBody<'get_site_metadata'>>({
      commit: EXAMPLE_SHA1,
      deployed_sha: EXAMPLE_SHA1,
      read_only: false,
      cdn_base: 'https://static.crates.io',
    });
  }),
];
