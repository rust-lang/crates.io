import { http, HttpResponse } from 'msw';

export default [
  // By default, crates have no RustSec advisories. Individual tests can override
  // this handler to return advisories for a specific crate.
  http.get('https://rustsec.org/packages/:crateId.json', () => {
    return HttpResponse.text('not found', { status: 404 });
  }),
];
