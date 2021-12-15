import { inject as service } from '@ember/service';
import Component from '@glimmer/component';

import { didCancel } from 'ember-concurrency';

import { simplifyUrl } from './crate-sidebar/link';

export default class CrateSidebar extends Component {
  @service playground;
  @service sentry;

  get showHomepage() {
    let { repository, homepage } = this.args.crate;
    return homepage && (!repository || simplifyUrl(repository) !== simplifyUrl(homepage));
  }

  get tomlSnippet() {
    return `${this.args.crate.name} = "${this.args.version.num}"`;
  }

  get playgroundLink() {
    let playgroundCrates = this.playground.crates;
    if (!playgroundCrates) return;

    let playgroundCrate = playgroundCrates.find(it => it.name === this.args.crate.name);
    if (!playgroundCrate) return;

    return `https://play.rust-lang.org/?edition=2018&code=use%20${playgroundCrate.id}%3B%0A%0Afn%20main()%20%7B%0A%20%20%20%20%2F%2F%20try%20using%20the%20%60${playgroundCrate.id}%60%20crate%20here%0A%7D`;
  }

  get canHover() {
    return window?.matchMedia('(hover: hover)').matches;
  }

  constructor() {
    super(...arguments);

    // load Rust Playground crates list, if necessary
    this.playground.loadCrates().catch(error => {
      if (!(didCancel(error) || error.isServerError || error.isNetworkError)) {
        // report unexpected errors to Sentry
        this.sentry.captureException(error);
      }
    });
  }
}
