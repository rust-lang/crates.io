{{page-title 'Dashboard'}}

<PageHeader class="header">
  <h1>My Dashboard</h1>
  <div class="stats">
    <div class='downloads'>
      {{svg-jar "download" class=(scoped-class "header-icon")}}
      <span class='num'>{{format-num this.myStats.total_downloads}}</span>
      <span class="stats-label text--small">Total Downloads</span>
    </div>
  </div>
</PageHeader>

<div class="my-info">
  <div class="my-crate-lists">
    <div class="header">
      <h2>
        {{svg-jar "my-packages"}}
        My Crates
      </h2>

      {{#if this.hasMoreCrates}}
        <LinkTo @route="me.crates" class="my-crates-link">Show all</LinkTo>
      {{/if}}
    </div>
    <CrateDownloadsList @crates={{this.visibleCrates}} />

    <div class='header'>
      <h2>
        {{svg-jar "following"}}
        Following
      </h2>

      {{#if this.hasMoreFollowing}}
        <LinkTo @route="me.following" class="followed-crates-link">Show all</LinkTo>
      {{/if}}
    </div>
    <CrateDownloadsList @crates={{this.visibleFollowing}} />
  </div>

  <div class="my-feed">
    <h2>
      {{svg-jar "latest-updates"}}
      Latest Updates
    </h2>

    <div class="feed">
      <ul class="feed-list" data-test-feed-list>
        {{#each this.myFeed as |version|}}
          <li class="feed-row">
            <LinkTo @route="crate.version" @models={{array version.crateName version.num}}>
              {{ version.crateName }}
              <span class="text--small">{{ version.num }}</span>
            </LinkTo>
            <span class="feed-date text--small">
              {{date-format-distance-to-now version.created_at addSuffix=true}}
            </span>
          </li>
        {{/each}}
      </ul>

      {{#if this.hasMore}}
        <div class="load-more">
          <button type="button" class="load-more-button" disabled={{this.loadMoreTask.isRunning}} {{on "click" (perform this.loadMoreTask)}}>
            Load More
            {{#if this.loadMoreTask.isRunning}}
              <LoadingSpinner />
            {{/if}}
          </button>
        </div>
      {{/if}}
    </div>
  </div>
</div>
