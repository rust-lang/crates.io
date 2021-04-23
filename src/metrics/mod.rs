pub use self::instance::InstanceMetrics;
pub use self::service::ServiceMetrics;

#[macro_use]
mod macros;

mod instance;
mod service;

load_metric_type!(IntGauge as single);
load_metric_type!(IntCounter as single);
load_metric_type!(IntGaugeVec as vec);
