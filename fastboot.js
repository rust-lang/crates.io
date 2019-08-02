/* eslint-disable no-console */

'use strict';

const fs = require('fs');
const FastBootAppServer = require('fastboot-app-server');

// because fastboot-app-server uses cluster, but it might change in future
const cluster = require('cluster');

function writeAppInitializedWhenReady() {
    let timeout;

    timeout = setInterval(function() {
        console.log('waiting backend');
        if (fs.existsSync('/tmp/backend-initialized')) {
            console.log('backend is up. let heroku know the app is ready');
            fs.writeFileSync('/tmp/app-initialized', 'hello');
            clearInterval(timeout);
        } else {
            console.log('backend is still not up');
        }
    }, 1000);
}

let server = new FastBootAppServer({
    distPath: 'dist',
    port: 9000,
});

if (!cluster.isWorker) {
    writeAppInitializedWhenReady();
}

server.start();
