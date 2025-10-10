<template>
  <h1>Something Went Wrong!</h1>
  <h5 data-test-error-message>{{@controller.model.message}}</h5>
  <pre class='terminal'>
  {{@controller.model.stack}}
</pre>
</template>
