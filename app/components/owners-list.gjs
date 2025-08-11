import Component from '@glimmer/component';

export default class VersionRow extends Component {
  get showDetailedList() {
    return this.args.owners.length <= 5;
  }
}

<ul
  role="list"
  class="list {{if this.showDetailedList "detailed"}}"
  data-test-owners="{{if this.showDetailedList "detailed" "basic"}}"
>
  {{#each @owners as |owner|}}
    <li class="{{if (eq owner.kind "team") "team"}}">
      <LinkTo
        @route={{owner.kind}}
        @model={{owner.login}}
        class="link"
        data-test-owner-link={{owner.login}}
      >
        <UserAvatar @user={{owner}} @size="medium-small" class="avatar" aria-hidden="true" />
        <span class="name {{unless this.showDetailedList "sr-only"}}">{{or owner.display_name owner.name owner.login}}</span>
      </LinkTo>
    </li>
  {{/each}}
</ul>