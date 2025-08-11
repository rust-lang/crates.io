<header class="header {{if @hero "hero"}}">
  <div class="header-inner width-limit">
    <LinkTo @route="index" class="index-link">
      <img src="/assets/cargo.png" role="none" alt="" class="logo">
      <h1>crates.io</h1>
    </LinkTo>

    <div class="search-form">
      <h1 class="hero-title">
        The Rust community&rsquo;s crate registry
      </h1>

      <SearchForm @size={{if @hero "big"}} @autofocus={{@hero}} />
    </div>

    <nav class='nav'>
      <ColorSchemeMenu class="color-scheme-menu" />

      <LinkTo @route="crates" @query={{hash letter=null page=1}} data-test-all-crates-link>
        Browse All Crates
      </LinkTo>
      <span class="sep">|</span>
      {{#if this.session.currentUser}}
        <Dropdown data-test-user-menu as |dd|>
          <dd.Trigger class="dropdown-button" data-test-toggle>
            {{#if this.session.isSudoEnabled}}
              <div data-test-wizard-hat class="wizard-hat">🧙</div>
            {{/if}}
            <UserAvatar @user={{this.session.currentUser}} @size="small" class="avatar" data-test-avatar />
            {{ this.session.currentUser.name }}
          </dd.Trigger>

          <dd.Menu class='current-user-links' as |menu|>
            <menu.Item><LinkTo @route='dashboard'>Dashboard</LinkTo></menu.Item>
            <menu.Item><LinkTo @route='settings' data-test-settings>Account Settings</LinkTo></menu.Item>
            <menu.Item><LinkTo @route='me.pending-invites'>Owner Invites</LinkTo></menu.Item>
            {{#if this.session.isAdmin}}
              <menu.Item class='sudo'>
                {{#if this.session.isSudoEnabled}}
                  <button data-test-disable-admin-actions class='sudo-menu-item button-reset' type='button' {{on 'click' this.disableSudo}}>
                    Disable admin actions
                    <div class='expires-in'>expires at {{date-format this.session.sudoEnabledUntil 'HH:mm'}}</div>
                  </button>
                {{else}}
                  <button data-test-enable-admin-actions class='sudo-menu-item button-reset' type='button' {{on 'click' this.enableSudo}}>
                    Enable admin actions
                  </button>
                {{/if}}
              </menu.Item>
            {{/if}}
            <menu.Item class='menu-item-with-separator'>
              <button
                type="button"
                disabled={{this.session.logoutTask.isRunning}}
                class="logout-menu-item button-reset"
                data-test-logout-button
                {{on "click" (perform this.session.logoutTask)}}
              >
                {{#if this.session.logoutTask.isRunning}}
                  <LoadingSpinner class="spinner"/>
                {{/if}}
                Sign Out
              </button>
            </menu.Item>
          </dd.Menu>
        </Dropdown>
      {{else}}
        <button
          type="button"
          disabled={{this.session.loginTask.isRunning}}
          class="login-button button-reset"
          data-test-login-button
          {{on "click" (perform this.session.loginTask)}}
        >
          {{#if this.session.loginTask.isRunning}}
            <LoadingSpinner class="spinner"/>
          {{else}}
            {{svg-jar "lock" class=(scoped-class "login-icon")}}
          {{/if}}
          Log in with GitHub
        </button>
      {{/if}}
    </nav>

    <div class='menu'>
      <ColorSchemeMenu class="color-scheme-menu" />

      <Dropdown as |dd|>
        <dd.Trigger class="dropdown-button">
          Menu
        </dd.Trigger>
        <dd.Menu class="current-user-links" as |menu|>
          <menu.Item><LinkTo @route="crates">Browse All Crates</LinkTo></menu.Item>
          {{#if this.session.currentUser}}
            <menu.Item><LinkTo @route="dashboard">Dashboard</LinkTo></menu.Item>
            <menu.Item><LinkTo @route="settings" data-test-me-link>Account Settings</LinkTo></menu.Item>
            <menu.Item><LinkTo @route="me.pending-invites">Owner Invites</LinkTo></menu.Item>
            <menu.Item class="menu-item-with-separator">
              <button
                type="button"
                disabled={{this.session.logoutTask.isRunning}}
                class="logout-menu-item button-reset"
                {{on "click" (perform this.session.logoutTask)}}
              >
                {{#if this.session.logoutTask.isRunning}}
                  <LoadingSpinner class="spinner"/>
                {{/if}}
                Sign Out
              </button>
            </menu.Item>
          {{else}}
            <menu.Item>
              <button
                type="button"
                disabled={{this.session.loginTask.isRunning}}
                class="login-menu-item button-reset"
                {{on "click" (perform this.session.loginTask)}}
              >
                {{#if this.session.loginTask.isRunning}}
                  <LoadingSpinner class="spinner"/>
                {{/if}}
                Log in with GitHub
              </button>
            </menu.Item>
          {{/if}}
        </dd.Menu>
      </Dropdown>
    </div>
  </div>
</header>