import { service } from '@ember/service';
import Component from '@glimmer/component';

export default class extends Component {
  @service colorScheme;
}

{{!--
  This component renders raw HTML. Be very careful with this since it
  can enable cross-site scripting attacks!
--}}
<TextContent
  ...attributes
  {{highlight-syntax selector="pre > code:not(.language-mermaid)"}}
  {{update-source-media this.colorScheme.resolvedScheme}}
  {{render-mermaids}}
>
  {{html-safe @html}}
</TextContent>