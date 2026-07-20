import { http, HttpResponse } from 'msw';

export default [
  http.head('https://raw.githubusercontent.com/:owner/:projec/HEAD/.github/workflows/:workflow_filename', () => {
    return new HttpResponse(null, { status: 404 });
  }),
];
