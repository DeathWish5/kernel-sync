use lock::future_mutex::FutureMutex as Mutex;
use std::{sync::Arc, vec};
// use tokio::task::{yield_now, JoinSet};

async fn handle1(x: Arc<Mutex<i32>>, loop_cnt: i32) {
    for _ in 0..loop_cnt {
        let mut guard = x.lock().await;
        *guard += 1;
    }
}

#[tokio::test]
async fn mutex_test1() {
    let x = Arc::new(Mutex::new(0));
    let coroutine_cnt = 10;
    let loop_cnt = 500;
    let mut coroutines = vec![];
    for _ in 0..coroutine_cnt {
        let x_cloned = x.clone();
        coroutines.push(tokio::spawn(handle1(x_cloned, loop_cnt)));
    }
    for coroutine in coroutines {
        tokio::join!(coroutine).0.unwrap();
    }
    assert_eq!(*x.lock().await, coroutine_cnt * loop_cnt);
}

// lazy_static::lazy_static! {
//     pub static ref DATA: Mutex<usize> = Mutex::new(0);
// }

// async fn handle2(n: usize) -> usize {
//     println!("task[{}] IN", n);
//     loop {
//         println!("task[{}] try lock", n);
//         let mut data = DATA.lock().await;
//         println!("task[{}] get lock data = {}", n, *data);
//         if *data == n {
//             *data += 1;
//             break;
//         }
//         yield_now().await;
//         drop(data);
//         yield_now().await;
//     }
//     println!("task[{}] finished", n);
//     n
// }

// #[tokio::test]
// async fn mutex_test2() {
//     let mut set = JoinSet::new();
//     for i in [6, 4, 2, 0] {
//         set.spawn(handle2(i));
//     }
//     for i in [1, 3, 5, 7] {
//         set.spawn(handle2(i));
//     }
//     println!("spawn over");
//     for i in 0..7 {
//         assert_eq!(i, set.join_one().await.unwrap().unwrap());
//     }
//     assert_eq!(*DATA.lock().await, 8);
// }
