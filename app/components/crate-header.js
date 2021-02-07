import { inject as service } from '@ember/service';
import Component from '@glimmer/component';

export default class CrateHeader extends Component {
  @service session;

  get documentationLink() {
    return this.args.version?.documentationLink ?? this.args.crate.documentation;
  }
}
