import { http, HttpResponse } from 'msw';

export default [
  http.head('https://gitlab.com/:owner/:project/-/raw/HEAD/:workflow_path', () => {
    return new HttpResponse(null, { status: 404 });
  }),
];
