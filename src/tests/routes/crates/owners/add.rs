use crate::builders::CrateBuilder;
use crate::util::{RequestHelper, TestApp};

// This is testing Cargo functionality! ! !
// specifically functions modify_owners and add_owners
// which call the `PUT /crates/:crate_id/owners` route
#[test]
fn test_cargo_invite_owners() {
    let (app, _, owner) = TestApp::init().with_user();

    let new_user = app.db_new_user("cilantro");
    app.db(|conn| {
        CrateBuilder::new("guacamole", owner.as_model().id).expect_build(conn);
    });

    #[derive(Serialize)]
    struct OwnerReq {
        owners: Option<Vec<String>>,
    }
    #[derive(Deserialize, Debug)]
    struct OwnerResp {
        // server must include `ok: true` to support old cargo clients
        ok: bool,
        msg: String,
    }

    let body = serde_json::to_string(&OwnerReq {
        owners: Some(vec![new_user.as_model().gh_login.clone()]),
    });
    let json: OwnerResp = owner
        .put("/api/v1/crates/guacamole/owners", body.unwrap().as_bytes())
        .good();

    // this ok:true field is what old versions of Cargo
    // need - do not remove unless you're cool with
    // dropping support for old versions
    assert!(json.ok);
    // msg field is what is sent and used in updated
    // version of cargo
    assert_eq!(
        json.msg,
        "user cilantro has been invited to be an owner of crate guacamole"
    )
}
