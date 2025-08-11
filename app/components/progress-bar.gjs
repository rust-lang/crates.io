import { service } from '@ember/service';
import Component from '@glimmer/component';

export default class extends Component {
  @service progress;
}

<div class="progress-bar" style={{this.progress.style}}></div>