# crates_io_pagerduty

This package contains the code necessary to interact with the PagerDuty API.

The crates.io on-call team uses PagerDuty to get notified about incidents. This
package contains a `PagerdutyClient` struct that can be configured with an API
token and service key to then trigger, acknowledge, and resolve incidents.
