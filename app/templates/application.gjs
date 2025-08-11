import HeadLayout from 'ember-cli-head/components/head-layout';
import NotificationContainer from 'ember-cli-notifications/components/notification-container';
import pageTitle from 'ember-page-title/helpers/page-title';
import rememberDocumentScroll from 'memory-scroll/modifiers/remember-document-scroll';

import Footer from 'crates-io/components/footer';
import Header from 'crates-io/components/header';
import ProgressBar from 'crates-io/components/progress-bar';
import setColorScheme from 'crates-io/helpers/set-color-scheme';
<template>
  <HeadLayout />

  {{pageTitle 'crates.io: Rust Package Registry' separator=' - ' prepend=true}}
  {{setColorScheme @controller.colorScheme.scheme}}

  <ProgressBar />
  <NotificationContainer @position='top-right' />
  <div id='tooltip-container'></div>

  <Header @hero={{@controller.isIndex}} />

  <main class='main' {{rememberDocumentScroll key=@controller.router.currentURL}}>
    <div class='inner-main width-limit'>
      {{outlet}}
    </div>
  </main>

  <Footer />
</template>
