import { hash } from '@ember/helper';

import NavTabsTab from 'crates-io/components/nav-tabs/tab';
<template>
  <nav ...attributes>
    <ul class='list'>
      {{yield (hash Tab=(component NavTabsTab))}}
    </ul>
  </nav>
</template>
