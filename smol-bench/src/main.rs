/// Implement an asynchroneous task benchchmark for smol runtime

use std::time::{Duration, Instant};
use smol::{io, channel::* };
use async_task::Task;
use async_executor::Executor;
use futures_lite::{future, future::Zip};
use shared::{AppSettings, TaskId};

/// Task that store instant and call another task through a given channel
/// Loop @LOOP_CNT times
async fn task_generic(tx : Sender<()>, rx : Receiver<()>, trig : Option<Receiver<()>>, id : crate::TaskId, settings : AppSettings) -> Vec<(TaskId, u128)> {
    //Init
    let mut iter : usize = 0;
    let mut clock : Vec<(crate::TaskId, u128)> = Vec::with_capacity(settings.limit);
    let instant: Instant  = Instant::now();
    let is_first: bool = trig.is_some();

    // Loop
    while iter < settings.limit {
        if iter == 0 && is_first {
            _ = trig.as_ref().unwrap().recv().await; }
        else {
            _ = tx.send(()).await; }
        clock.push((id, instant.elapsed().as_nanos()));
        iter += 1;

        if settings.display_iter {
            let name_task: &str = id.nameof();
            println!("{name_task} - {iter}");
        }
        _ = rx.recv().await;
    }
    clock
}

/// Benchmark the task switch through "**smol**" runtime.\
/// Return `()` or io error occured during file access.
fn main() -> io::Result<()> {
    // Read app arguments
    let settings : AppSettings = shared::read_app_argument();

    // Try to set thread priority
    thread_priority::set_current_thread_priority(thread_priority::ThreadPriority::Max).unwrap_or(());

    //Channel to synchronise tasks
    let (tx1, rx1) : (Sender<()>, Receiver<()>) = async_channel::bounded(1);
    let (tx2, rx2) : (Sender<()>, Receiver<()>) = async_channel::bounded(1);
    let (tx_trig, rx_trig) : (Sender<()>, Receiver<()>)  = async_channel::bounded(1);

    // Build task
    let exec = Executor::new();
    let zip: Zip<Task<Vec<(TaskId, u128)>>, Task<Vec<(TaskId, u128)>>> = future::zip(
        exec.spawn(task_generic(tx1, rx2, Some(rx_trig), TaskId::A, settings.clone())),
        exec.spawn(task_generic(tx2, rx1, None, TaskId::B, settings.clone()))
    );

    // Run tasks and print result in a file
    let r: (Vec<(TaskId, u128)>, Vec<(TaskId, u128)>) = future::block_on(async {
        let future_r = exec.run(zip);
        std::thread::sleep(Duration::from_micros(shared::LAZY_START));
        _ = tx_trig.send(()).await;
        future_r.await
    });

    if settings.output_file.is_empty() {
        shared::format_str(&r.0, &r.1 ,"smol_diff.csv")?; }
    else {
        shared::format_str(&r.0, &r.1 ,&settings.output_file.as_str())?; }
    Ok(())
}
