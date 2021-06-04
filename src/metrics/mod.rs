pub use self::instance::InstanceMetrics;
pub use self::log_encoder::LogEncoder;
pub use self::service::ServiceMetrics;

#[macro_use]
mod macros;

mod instance;
mod log_encoder;
mod service;

load_metric_type!(IntGauge as single);
load_metric_type!(IntCounter as single);
load_metric_type!(IntGaugeVec as vec);
