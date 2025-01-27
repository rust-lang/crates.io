import { importSync, isDevelopingApp, isTesting, macroCondition } from '@embroider/macros';

export default {
  name: 'ember-cli-mirage',
  initialize(application) {
    // `macroCondition(isDevelopingApp() || isTesting())` should work as well,
    // but it failed to build correctly on CI, so we duplicate it into two
    // conditions. Since we will be dropping `ember-cli-mirage` soon anyway,
    // this is good enough for now.
    if (macroCondition(isDevelopingApp())) {
      let startMirage = importSync('ember-cli-mirage/start-mirage').default;
      let ENV = importSync('../config/environment').default;
      let makeServer = importSync('../mirage/config').default;

      application.register('mirage:make-server', makeServer, {
        instantiate: false,
      });

      if (window.startMirage) {
        startMirage(application.__container__, { makeServer, env: ENV });
      }
    } else if (macroCondition(isTesting())) {
      let startMirage = importSync('ember-cli-mirage/start-mirage').default;
      let ENV = importSync('../config/environment').default;
      let makeServer = importSync('../mirage/config').default;

      application.register('mirage:make-server', makeServer, {
        instantiate: false,
      });

      if (window.startMirage) {
        startMirage(application.__container__, { makeServer, env: ENV });
      }
    }
  },
};
