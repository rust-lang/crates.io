import Component from '@glimmer/component';

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

<div ...attributes>
  <h2 class="title" data-test-title>{{@title}}</h2>
  <div class="content">
    {{#if this.isDocsRs}}
      {{svg-jar "docs-rs" class=(scoped-class "icon") data-test-icon="docs-rs"}}
    {{else if this.isGitHub}}
      {{svg-jar "github" class=(scoped-class "icon") data-test-icon="github"}}
    {{else}}
      {{svg-jar "link" class=(scoped-class "icon") data-test-icon="link"}}
    {{/if}}

    <a href={{@url}} class="link" data-test-link>
      {{this.text}}
    </a>
  </div>
</div>