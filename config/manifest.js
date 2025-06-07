'use strict';

module.exports = function (/* environment, appConfig */) {
  return {
    name: 'crates.io: Rust Package Registry',
    short_name: 'crates.io',
    description: 'crates.io is the default crate host for Rust.',
    start_url: '/',
    display: 'standalone',
    background_color: '#3b6837',
    icons: [
      {
        src: '/assets/cargo.png',
        sizes: '227x227',
        type: 'image/png',
      },
    ],
    ms: {
      tileColor: '#3b6837',
    },
  };
};
