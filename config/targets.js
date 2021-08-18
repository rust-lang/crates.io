module.exports = {
  browsers: [
    // These are the browsers we actually are attempting to support:
    'last 2 Chrome versions',
    'last 1 Firefox version',
    'Firefox ESR',
    'last 1 Safari version',
    'last 1 iOS version',
    'last 1 Edge version',
    'last 1 UCAndroid version',
    'last 1 years',
    // Edge for Android is currently (2021-08-18) using an outdated Chromium version (v77),
    // which is not recognized by `browserslist` and `caniuse`. This can be removed once
    // Edge for Android has promoted their "Edge for Android Beta" app to production, which
    // is using a more up-to-date Chromium version.
    // (see https://github.com/rust-lang/crates.io/issues/3838)
    'Chrome 77',
  ],
};
