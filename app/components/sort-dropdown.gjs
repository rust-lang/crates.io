<Dropdown as |dd|>
  <dd.Trigger class="trigger" data-test-current-order>
    {{svg-jar "sort" class=(scoped-class "icon")}}
    {{@current}}
  </dd.Trigger>

  <dd.Menu as |menu|>
    {{yield (hash Option=(component "sort-dropdown/option" menu=menu))}}
  </dd.Menu>
</Dropdown>