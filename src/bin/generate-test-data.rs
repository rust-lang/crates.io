// Generates test data
// Usage:
//      cargo run --bin generate-test-data

#![deny(warnings)]
#![feature(convert)]
extern crate cargo_registry;
extern crate postgres;
extern crate rand;

use std::env;
use std::io::prelude::*;

use cargo_registry::{Crate, User};
use rand::{thread_rng, Rng};

const STR_LEN: usize = 5;
const COUNT: i32 = 10;

fn main() {
    let conn = postgres::Connection::connect(&env("DATABASE_URL")[..],
                                      &postgres::SslMode::None).unwrap();
    let tx = conn.transaction().unwrap();
    add_crates(&tx, COUNT);
    tx.set_commit();
    tx.finish().unwrap();
}

fn env(s: &str) -> String {
    match env::var(s).ok() {
        Some(s) => s,
        None => panic!("must have `{}` defined", s),
    }
}

fn add_user(conn: &postgres::Transaction, name: String, access_token: String,
            api_token: String) -> User {
    User::find_or_insert(conn, name.as_str(), None, None, None,
                         access_token.as_str(), api_token.as_str()).unwrap()
 }

fn add_crate(tx: &postgres::Transaction, name: String, user_id: i32) -> Crate {
    Crate::find_or_insert(tx, name.as_str(), user_id, &None, &None,
                          &None, &None, &[], &None, &None,
                          &None).unwrap()
}

fn add_crates(tx: &postgres::Transaction, num: i32) -> () {
    for _ in 1..(num+1) {
        let user = add_user(&tx, generate_name(STR_LEN), generate_name(STR_LEN),
                            generate_name(STR_LEN));
        let crate_name = generate_name(STR_LEN);
        add_crate(&tx, crate_name, user.id);
    }
}

fn generate_name(len: usize) -> String {
    thread_rng().gen_ascii_chars().take(len).collect()
}
