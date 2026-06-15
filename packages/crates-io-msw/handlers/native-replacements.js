import { http, HttpResponse } from 'msw';

const replacements = {
  lazy_static: {
    description:
      'The standard library provides `std::sync::LazyLock` (stable since Rust 1.80), which lets you replace the `lazy_static!` macro with a plain `static` and remove the dependency.',
    url: 'https://doc.rust-lang.org/std/sync/struct.LazyLock.html',
  },
};

export default [
  http.get('https://rust-lang.github.io/std-replacement-data/all.json', () => HttpResponse.json(replacements)),
];
