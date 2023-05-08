'use strict';

// Note that this is not _really_ a server in any meaningful sense.
//
// This module is imported by ember-cli when running the server task to discover
// any app-specific middleware that should be used. Since we do want to install
// some middleware to allow us to customise how requests are routed and proxied
// to development backends, we'll do the work to set that up from here.
//
// Unfortunately, the name `server` is hardcoded in ember-cli.
//
// The ecosystem norm is to define proxies in a `proxies` subdirectory, as set
// up by the relevant blueprint. However, doing so disables ember-cli-mirage by
// default, which isn't what we want, and we're only ever going to have one
// proxy anyway, so we'll just define the logic inline below.

// Prefixes that we want to proxy straight through to the backend.
//
// This list must be kept up to date with the nginx configuration _and_ the list
// of prefixes in `middleware::ember_html::serve_html`.
const proxyPaths = ['/api/'];

function installBackendProxy(app) {
  // Load the proxy backend from the environment.
  const proxyBackend = process.env.PROXY_BACKEND;

  // It's OK if no proxy backend is provided; we'll use data from the Mirage
  // static fixtures instead.
  if (!proxyBackend) {
    return;
  }

  const proxy = require('http-proxy').createProxyServer({
    target: proxyBackend,
    // Required for SSL/TLS to work with crates.io.
    changeOrigin: true,
  });

  proxy.on('error', function (err, req) {
    console.error(err, req.url);
  });

  for (const proxyPath of proxyPaths) {
    app.use(proxyPath, function (req, res) {
      // Reconstruct the full relative URL, being careful not to accidentally
      // introduce a double slash.
      req.url = proxyPath.endsWith('/') && req.url.startsWith('/') ? proxyPath + req.url.slice(1) : proxyPath + req.url;

      proxy.web(req, res);
    });
  }
}

module.exports = function (app) {
  // Log proxy requests.
  const morgan = require('morgan');
  app.use(morgan('dev'));

  installBackendProxy(app);
};
