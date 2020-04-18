//! Endpoints that provide badges based on crate metadata

use crate::controllers::frontend_prelude::*;

use crate::models::{Badge, Crate, CrateBadge, MaintenanceStatus};
use crate::schema::*;

use conduit::{Body, Response};

/// Handles the `GET /crates/:crate_id/maintenance.svg` route.
pub fn maintenance(req: &mut dyn RequestExt) -> EndpointResult {
    let name = &req.params()["crate_id"];
    let conn = req.db_read_only()?;

    let krate = Crate::by_name(name).first::<Crate>(&*conn);
    if krate.is_err() {
        let response = Response::builder().status(404).body(Body::empty()).unwrap();

        return Ok(response);
    }

    let krate = krate.unwrap();

    let maintenance_badge = CrateBadge::belonging_to(&krate)
        .select((badges::crate_id, badges::all_columns))
        .load::<CrateBadge>(&*conn)?
        .into_iter()
        .find(|cb| matches!(cb.badge, Badge::Maintenance { .. }));

    if maintenance_badge.is_none() {
        return Ok(req.redirect(
            "https://img.shields.io/badge/maintenance-unknown-lightgrey.svg".to_owned(),
        ));
    }

    let status = match maintenance_badge {
        Some(CrateBadge {
            badge: Badge::Maintenance { status },
            ..
        }) => Some(status),
        _ => None,
    };

    let status = status.unwrap();

    let message = match status {
        MaintenanceStatus::ActivelyDeveloped => "actively--developed",
        MaintenanceStatus::PassivelyMaintained => "passively--maintained",
        MaintenanceStatus::AsIs => "as--is",
        MaintenanceStatus::None => "unknown",
        MaintenanceStatus::Experimental => "experimental",
        MaintenanceStatus::LookingForMaintainer => "looking--for--maintainer",
        MaintenanceStatus::Deprecated => "deprecated",
    };

    let color = match status {
        MaintenanceStatus::ActivelyDeveloped => "brightgreen",
        MaintenanceStatus::PassivelyMaintained => "yellowgreen",
        MaintenanceStatus::AsIs => "yellow",
        MaintenanceStatus::None => "lightgrey",
        MaintenanceStatus::Experimental => "blue",
        MaintenanceStatus::LookingForMaintainer => "orange",
        MaintenanceStatus::Deprecated => "red",
    };

    let url = format!(
        "https://img.shields.io/badge/maintenance-{}-{}.svg",
        message, color
    );
    Ok(req.redirect(url))
}
