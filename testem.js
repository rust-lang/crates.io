'use strict';

module.exports = {
  test_page: 'tests/index.html?hidepassed',
  disable_watching: true,
  parallel: -1,
  launch_in_ci: ['Chrome'],
  launch_in_dev: ['Chrome'],
  browser_start_timeout: 120,
  browser_args: {
    Chrome: {
      mode: 'ci',
      args: ['--disable-gpu', '--headless', '--remote-debugging-port=9222', '--window-size=1440,900'],
    },
  },
};
