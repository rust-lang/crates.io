import { http } from '../utils/openapi-http.js';

const EXAMPLE_SHA1 = '5048d31943118c6d67359bd207d307c854e82f45';

export default [
  http.get('/api/v1/site_metadata', ({ response }) => {
    return response(200).json({
      commit: EXAMPLE_SHA1,
      deployed_sha: EXAMPLE_SHA1,
      read_only: false,
      cdn_base: 'https://static.crates.io',
    });
  }),
];
