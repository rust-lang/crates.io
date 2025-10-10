import { LinkTo } from '@ember/routing';
import { service } from '@ember/service';
import Component from '@glimmer/component';

import svgJar from 'ember-svg-jar/helpers/svg-jar';

export default class Footer extends Component {
  @service pristineQuery;

  get pristineSupportQuery() {
    let params = this.pristineQuery.paramsFor('support');
    return params;
  }

  <template>
    <footer class='footer'>
      <div class='content width-limit'>
        <div>
          <h1>Rust</h1>
          <ul role='list'>
            <li><a href='https://www.rust-lang.org/'>rust-lang.org</a></li>
            <li><a href='https://foundation.rust-lang.org/'>Rust Foundation</a></li>
            <li><a href='https://www.rust-lang.org/governance/teams/dev-tools#team-crates-io'>The crates.io team</a></li>
          </ul>
        </div>

        <div>
          <h1>Get Help</h1>
          <ul role='list'>
            <li><a href='https://doc.rust-lang.org/cargo/'>The Cargo Book</a></li>
            <li><LinkTo @route='support' @query={{this.pristineSupportQuery}}>Support</LinkTo></li>
            <li><a href='https://status.crates.io/'>System Status</a></li>
            <li><a href='https://github.com/rust-lang/crates.io/issues/new/choose'>Report a bug</a></li>
          </ul>
        </div>

        <div>
          <h1>Policies</h1>
          <ul role='list'>
            <li><LinkTo @route='policies'>Usage Policy</LinkTo></li>
            <li><LinkTo @route='policies.security'>Security</LinkTo></li>
            <li><a href='https://foundation.rust-lang.org/policies/privacy-policy/'>Privacy Policy</a></li>
            <li><a href='https://www.rust-lang.org/policies/code-of-conduct'>Code of Conduct</a></li>
            <li><LinkTo @route='data-access'>Data Access</LinkTo></li>
          </ul>
        </div>

        <div>
          <h1>Social</h1>
          <ul role='list'>
            <li><a href='https://github.com/rust-lang/crates.io/'>{{svgJar 'github'}} rust-lang/crates.io</a></li>
            <li><a href='https://rust-lang.zulipchat.com/#streams/318791/t-crates-io'>{{svgJar 'zulip'}}
                #t-crates-io</a></li>
            <li><a href='https://twitter.com/cratesiostatus'>{{svgJar 'twitter'}} @cratesiostatus</a></li>
          </ul>
        </div>
      </div>
    </footer>
  </template>
}
