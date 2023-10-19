/// Implement an asynchroneous task benchchmark for smol runtime

use std::{io::prelude::*, time::{Duration, Instant}, fs::File, env};
use smol::{io, channel::* };
use async_task::Task;
use async_executor::Executor;
use futures_lite::{future, future::Zip};

const DEFAULT_LOOP_CNT : usize = 1000;
const LAZY_START : u64 = 2000;

#[derive(Clone, Copy, PartialEq, Eq)]
enum TaskId {
    A = 0,
    B,
}

/// Task that store instant and call another task through a given channel
/// Loop @LOOP_CNT times
async fn task_generic(tx : Sender<()>, rx : Receiver<()>, trig : Option<Receiver<()>>, id : crate::TaskId, limit : usize) -> Vec<(TaskId, u128)> {
    //Init
    let mut iter : usize = 0;
    let mut clock : Vec<(crate::TaskId, u128)> = Vec::with_capacity(limit);
    let instant: Instant  = Instant::now();
    let is_first: bool = trig.is_some();

    // Loop
    while iter < limit {
        if iter == 0 && is_first {
            _ = trig.as_ref().unwrap().recv().await; }
        else {
            _ = tx.send(()).await; }
        clock.push((id, instant.elapsed().as_nanos()));
        iter += 1;
        _ = rx.recv().await;
    }

    clock
}

/// Benchmark the task switch through "**smol**" runtime.\
/// Return `()` or io error occured during file access.
fn main() -> io::Result<()> {

    // Read app arguments
    let mut limit : usize = DEFAULT_LOOP_CNT;
    let vargs : Vec<String> = env::args().into_iter().collect();
    for i in 0..vargs.len()
    {
        if (vargs[i] == "-l" || vargs[i] == "-limit") && i + 1 < vargs.len() {
            limit = vargs[i + 1].parse::<usize>().unwrap_or(DEFAULT_LOOP_CNT);
        }
    }

    // Try to set thread priority
    thread_priority::set_current_thread_priority(thread_priority::ThreadPriority::Max).unwrap_or(());

    //Channel to synchronise tasks
    let (tx1, rx1) : (Sender<()>, Receiver<()>) = async_channel::bounded(1);
    let (tx2, rx2) : (Sender<()>, Receiver<()>) = async_channel::bounded(1);
    let (tx_trig, rx_trig) : (Sender<()>, Receiver<()>)  = async_channel::bounded(1);

    // Build task
    let exec = Executor::new();
    let zip: Zip<Task<Vec<(TaskId, u128)>>, Task<Vec<(TaskId, u128)>>> = future::zip(
        exec.spawn(task_generic(tx1, rx2, Some(rx_trig), TaskId::A, limit)),
        exec.spawn(task_generic(tx2, rx1, None, TaskId::B, limit))
    );

    // Run tasks and print result in a file
    let r: (Vec<(TaskId, u128)>, Vec<(TaskId, u128)>) = future::block_on(async {
        let future_r = exec.run(zip);
        std::thread::sleep(Duration::from_micros(LAZY_START));
        _ = tx_trig.send(()).await;
        future_r.await
    });
    format_str(&r.0, &r.1 ,"smol_diff.csv")?;
    Ok(())
}

/// Concatenate per line the "instant" set by a task A and B. Also compute the absolute difference between the time point of the two task,
/// and display it under the colum *Diff*.
///
/// TODO: Rewrite this function with "variadic arguments"
fn format_str(col_a : &Vec<(TaskId, u128)>,  col_b : &Vec<(TaskId, u128)>, title : &str) -> std::io::Result<()>
{
    if col_a.len() == col_b.len()
    {
        let mut str_diff = String::from("Producer Consumer Diff\n");
        let mut file_diff: File = File::create(title)?;
        for i in 0..col_a.len() {
            str_diff += (col_a[i].1.to_string() + " ").as_ref();
            str_diff += (col_b[i].1.to_string() + " ").as_ref();
            str_diff += ((col_a[i].1.abs_diff(col_b[i].1)).to_string() + "\n").as_str();
        }
        file_diff.write_all(str_diff.as_bytes())?;
    }
    Ok(())
}
