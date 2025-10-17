use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_GH_ID: AtomicUsize = AtomicUsize::new(1);

pub fn next_gh_id() -> i32 {
    NEXT_GH_ID.fetch_add(1, Ordering::SeqCst) as i32
}
