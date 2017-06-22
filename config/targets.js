/* eslint-env node */
module.exports = {
  browsers: [
    // We don't actually intend to support Safari 4, but PhantomJS uses an
    // old version of webkit. There's no browserslist string for PhantomJS,
    // but this simulates it so that autoprefixer works for running tests.
    'Safari 4',

    // These are the browsers we actually are attempting to support:
    'last 2 Chrome versions',
    'last 1 Firefox version',
    'Firefox ESR',
    'last 1 Safari version',
    'last 1 iOS version',
    'last 1 Edge version',
    'last 1 UCAndroid version'
  ]
};
