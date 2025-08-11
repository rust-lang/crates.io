<HeadLayout />

{{page-title "crates.io: Rust Package Registry" separator=' - ' prepend=true}}
{{set-color-scheme this.colorScheme.scheme}}

<ProgressBar/>
<NotificationContainer @position="top-right"/>
<div id="tooltip-container"></div>

<Header @hero={{this.isIndex}} />

<main class="main" {{remember-document-scroll key=this.router.currentURL}}>
  <div class="inner-main width-limit">
  {{outlet}}
  </div>
</main>

<Footer/>