use conduit_test::MockRequest;
use conduit::{mod, Handler};

#[test]
fn auth_gives_a_token() {
    #[deriving(Decodable)]
    struct Foo {
        url: String,
        state: String,
    }

    let middle = ::app();
    let mut req = MockRequest::new(conduit::Get, "/authorize_url");
    let mut response = t_resp!(middle.call(&mut req));
    let json: Foo = ::json(&mut response);
    assert!(json.url.as_slice().contains(json.state.as_slice()));
}
