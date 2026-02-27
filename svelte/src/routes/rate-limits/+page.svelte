<script lang="ts">
  import PageHeader from '$lib/components/PageHeader.svelte';
  import TextContent from '$lib/components/TextContent.svelte';
</script>

<PageHeader title="Publishing Rate Limits" />

<TextContent boxed>
  <p>
    Crates.io has rate limits on publishes of new crates and publishes of new versions of existing crates to mitigate
    spamming and overloading of the crates.io service.
  </p>

  <p>
    Rate limiting is implemented using the <a href="https://en.wikipedia.org/wiki/Leaky_bucket#As_a_meter"
      >leaky bucket as a meter</a
    > algorithm. This allows for a burst of activity up to a threshold, and then sets a rate at which the allowance of further
    activity is earned back over time.
  </p>

  <p>Concretely, the rate limits for crates.io are:</p>

  <ul>
    <li>
      For brand new crate publishes, allow a burst of 5 new crates published at once by a single user account, with a
      rate limit of 1 crate every 10 minutes allowed after that burst
    </li>
    <li>
      For new versions of existing crates, allow a burst of 30 new versions published at once by a single user account,
      with a rate limit of 1 crate per minute allowed after that burst
    </li>
  </ul>

  <p>The rate limiting error message will give the time at which you may send the next publish.</p>

  <p>
    We are unlikely to grant exceptions to these rate limits, to preserve the performance of the system and quantity of
    crates published for everyone's benefit.
  </p>

  <p>Please plan your publishes according to these limits.</p>
</TextContent>
