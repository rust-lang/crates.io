import { http, HttpResponse } from 'msw';

export default [
  http.get('https://docs.rs/crate/:crate/:version/status.json', () => {
    return HttpResponse.json({});
  }),
];
