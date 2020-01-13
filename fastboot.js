/* eslint-env node */

'use strict';

const fs = require('fs');
const os = require('os');
const FastBootAppServer = require('fastboot-app-server');

// because fastboot-app-server uses cluster, but it might change in future
const cluster = require('cluster');

const morgan = require('morgan');

class LoggerWithoutTimestamp {
  constructor() {
    this.prefix = cluster.isMaster ? 'master' : 'worker';
  }
  writeLine() {
    this._write('info', Array.prototype.slice.apply(arguments));
  }

  writeError() {
    this._write('error', Array.prototype.slice.apply(arguments));
  }

  _write(level, args) {
    args[0] = `[${level}][${this.prefix}] ${args[0]}`;
    console.log.apply(console, args);
  }
}

function writeAppInitializedWhenReady(logger) {
  let timeout;

  timeout = setInterval(function() {
    logger.writeLine('waiting backend');
    if (fs.existsSync('/tmp/backend-initialized')) {
      logger.writeLine('backend is up. let heroku know the app is ready');
      fs.writeFileSync('/tmp/app-initialized', 'hello');
      clearInterval(timeout);
    } else {
      logger.writeLine('backend is still not up');
    }
  }, 1000);
}

var logger = new LoggerWithoutTimestamp();
logger.writeLine(`${os.cpus().length} cores available`);

let workerCount = process.env.WEB_CONCURRENCY || 1;
let logRequests = morgan(
  'at=info method=:method path=":url" ' +
    'request_id=:req[x-request-id] ' +
    'fwd=":req[x-real-ip]" ' +
    'user_agent=":req[user-agent]"',
);

let server = new FastBootAppServer({
  distPath: 'dist',
  port: 9000,
  ui: logger,
  workerCount: workerCount,

  // afterMiddleware won't be called since Fastboot's middleware
  // doesn't call next().
  beforeMiddleware: app => app.use(logRequests),
});

if (!cluster.isWorker) {
  writeAppInitializedWhenReady(logger);
}

server.start();
