import { http, HttpResponse } from 'msw';

export default [http.get('https://play.rust-lang.org/meta/crates', () => HttpResponse.json([]))];
