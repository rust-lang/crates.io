import { service } from '@ember/service';
import Component from '@glimmer/component';

import TextContent from 'crates-io/components/text-content';
import htmlSafe from 'crates-io/helpers/html-safe';
import highlightSyntax from 'crates-io/modifiers/highlight-syntax';
import renderMermaids from 'crates-io/modifiers/render-mermaids';
import updateSourceMedia from 'crates-io/modifiers/update-source-media';

export default class extends Component {
  <template>
    {{!
  This component renders raw HTML. Be very careful with this since it
  can enable cross-site scripting attacks!
}}
    <TextContent
      ...attributes
      {{highlightSyntax @html selector='pre > code:not(.language-mermaid)'}}
      {{updateSourceMedia @html this.colorScheme.resolvedScheme}}
      {{renderMermaids @html}}
    >
      {{htmlSafe @html}}
    </TextContent>
  </template>
  @service colorScheme;
}
