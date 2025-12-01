import Component from '@glimmer/component';

import scopedClass from 'ember-scoped-css/helpers/scoped-class';
import svgJar from 'ember-svg-jar/helpers/svg-jar';

export default class CrateSidebarLink extends Component {
  get text() {
    let { url } = this.args;
    return simplifyUrl(url);
  }

  get isDocsRs() {
    return this.text.startsWith('docs.rs/');
  }

  get isGitHub() {
    return this.text.startsWith('github.com/');
  }

  get isGitLab() {
    return this.text.startsWith('gitlab.com/');
  }

  get isCodeberg() {
    return this.text.startsWith('codeberg.org/');
  }

  <template>
    <div ...attributes>
      <h2 class='title' data-test-title>{{@title}}</h2>
      <div class='content'>
        {{#if this.isDocsRs}}
          {{svgJar 'docs-rs' class=(scopedClass 'icon') data-test-icon='docs-rs'}}
        {{else if this.isGitHub}}
          {{svgJar 'github' class=(scopedClass 'icon') data-test-icon='github'}}
        {{else if this.isGitLab}}
          {{svgJar 'gitlab' class=(scopedClass 'icon') data-test-icon='gitlab'}}
        {{else if this.isCodeberg}}
          {{svgJar 'codeberg' class=(scopedClass 'icon') data-test-icon='codeberg'}}
        {{else}}
          {{svgJar 'link' class=(scopedClass 'icon') data-test-icon='link'}}
        {{/if}}

        <a href={{@url}} class='link' data-test-link>
          {{this.text}}
        </a>
      </div>
    </div>
  </template>
}

export function simplifyUrl(url) {
  if (url.startsWith('https://')) {
    url = url.slice('https://'.length);
  }
  if (url.startsWith('www.')) {
    url = url.slice('www.'.length);
  }
  if (url.endsWith('/')) {
    url = url.slice(0, -1);
  }
  if (url.startsWith('github.com/') && url.endsWith('.git')) {
    url = url.slice(0, -4);
  }

  return url;
}
