extern crate green;
extern crate rustuv;

extern crate civet;
extern crate conduit;
extern crate "conduit-static" as conduit_static;

use std::os;

use civet::{Config, Server};
use conduit_static::Static;

fn main() {
    let handler = Static::new(os::getcwd());
    let _a = Server::start(Config { port: 8888, threads: 50 }, handler);

    wait_for_sigint();
}

fn wait_for_sigint() {
    use green::{SchedPool, PoolConfig, GreenTaskBuilder};
    use std::io::signal::{Listener, Interrupt};
    use std::task::TaskBuilder;

    let mut config = PoolConfig::new();
    config.event_loop_factory = rustuv::event_loop;

    let mut pool = SchedPool::new(config);
    TaskBuilder::new().green(&mut pool).spawn(proc() {
        let mut l = Listener::new();
        l.register(Interrupt).unwrap();
        l.rx.recv();
    });
    pool.shutdown();
}
