#![feature(async_closure)]

use lock::future_rwlock::FutureRwLock as RwLock;
use std::vec::Vec;
use std::sync::Arc;
use std::sync::mpsc::channel;
use tokio::task::yield_now;

#[tokio::test]
async fn rwlock_test1() {
    let arc = Arc::new(RwLock::new(0));
    let arc2 = arc.clone();
    let (tx, rx) = channel();
    let mut children = Vec::new();
    children.push(tokio::spawn(async move {
        let mut lock = arc2.write().await;
        for _ in 0..10 {
            let tmp = *lock;
            *lock = -1;
            yield_now();
            *lock = tmp + 1;
        }
        tx.send(()).unwrap();
    }));

    // Readers try to catch the writer in the act
    for _ in 0..5 {
        let arc3 = arc.clone();
        children.push(tokio::spawn(async move {
            let lock = arc3.read().await;
            assert!(*lock >= 0);
        }));
    }

    // Wait for children to pass their asserts
    for r in children {
        tokio::join!(r);
    }

    // Wait for writer to finish
    rx.recv().unwrap();
    let lock = arc.read().await;
    assert_eq!(*lock, 10);
}