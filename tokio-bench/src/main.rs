/// Implement an asynchroneous (and thread) task benchchmark for tokio runtime
use tokio;
use tokio::sync::Notify;
use std::{
    sync::{ Arc, Mutex, atomic::{ Ordering, AtomicUsize }, mpsc::{ channel, Sender, Receiver } },
    time::{ SystemTime, Instant, Duration, UNIX_EPOCH },
    io::prelude::*,
    fs::File,
    thread,
};
use futures_concurrency::future::Join;

static LOOP_CNT : i32 = 1000;
static LAZY_START : u64 = 2000;

#[derive(Clone, Copy)]
enum TaskId {
    A = 0,
    B,
}

/// Producer used in asynchroneous function
async fn task_generic(notify : Arc<Notify>, trigger : Option<Arc<Notify>>, id : TaskId) -> Vec<(TaskId, u128)> {
    //Init
    let mut iter : i32 = 0;
    let mut clock : Vec<(crate::TaskId, u128)> = Vec::with_capacity(LOOP_CNT.try_into().unwrap());
    let instant: Instant  = Instant::now();
    let is_first: bool = trigger.is_some();

    // Loop
    while iter < LOOP_CNT {
        if iter == 0 && is_first {
            trigger.as_ref().unwrap().notified().await; }
        else {
            notify.notified().await; }
        clock.push((id, instant.elapsed().as_nanos()));
        iter += 1;
        notify.notify_waiters();
    }

    clock
}

/// Producer or consumer used in synchrone function
fn thread_generic(tx : Sender<()>, rx : Receiver<()>, trigger : Option<Receiver<()>>, id : TaskId) -> Vec<(TaskId,u128)> {
    //Init
    let mut iter : i32 = 0;
    let mut clock : Vec<(crate::TaskId, u128)> = Vec::with_capacity(LOOP_CNT.try_into().unwrap());
    let instant: Instant = Instant::now();
    let is_first: bool = trigger.is_some();

    //Loop
    while iter < LOOP_CNT {
        if iter == 0 && is_first {
            _ = trigger.as_ref().unwrap().recv().unwrap(); }
        else {
            _ = rx.recv().unwrap(); }
        clock.push((id, instant.elapsed().as_nanos()));
        iter += 1;
        _ = tx.send(());
    }

    clock
}

#[tokio::main]
async fn main() -> std::io::Result<()> {

    // Try to set thread priority
    thread_priority::set_current_thread_priority(thread_priority::ThreadPriority::Max).unwrap_or(());

    //For async and threading
    let trigger = Arc::new(Notify::new());
    let notify1 = Arc::new(Notify::new());
    let notify2 = notify1.clone();

    //Tokio tasks
    let tasks = (
        tokio::task::spawn( task_generic(notify1, Some(trigger.clone()), TaskId::A )),
        tokio::task::spawn( task_generic(notify2, None, TaskId::B))
    ).join();
    std::thread::sleep(Duration::from_micros(LAZY_START));
    trigger.notify_one();
    let results: (Result<Vec<(TaskId, u128)>, tokio::task::JoinError>, Result<Vec<(TaskId, u128)>, tokio::task::JoinError>) = tasks.await;
    format_str(results.0.as_ref().unwrap(), results.1.as_ref().unwrap() ,"tokio_diff.csv")?;

    //For threading
    let (tx1, rx1) : (Sender<()>, Receiver<()>) = channel();
    let (tx2, rx2) : (Sender<()>, Receiver<()>) = channel();
    let (tx_trigger,rx_trigger) = channel();
    let threads = (
        thread::spawn( || thread_generic(tx1, rx2, Some(rx_trigger) ,TaskId::A)),
        thread::spawn( || thread_generic(tx2, rx1, None, TaskId::B)),
    );
    std::thread::sleep(Duration::from_micros(LAZY_START));
    _ = tx_trigger.send(());
    format_str(&threads.0.join().unwrap(), &threads.1.join().unwrap() ,"tokio_diff_thread.csv")?;

    Ok(())
}

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

// ================= DEPRECATED =====================


/// Producer used in asynchroneous function
async fn producer_task(notify : Arc<Notify>, clock_iter : Arc<AtomicUsize>, clock : Arc<Mutex<Vec<(TaskId, u128)>>> ) {
    let mut iter : i32 = 0;
    while iter < LOOP_CNT {
        if iter != 0 { notify.notified().await; }
        else { std::thread::sleep(Duration::from_micros(LAZY_START)); }
        let ns: u128 = SystemTime::now().duration_since(UNIX_EPOCH).expect("").as_nanos();
        clock_iter.fetch_add(1, Ordering::AcqRel);
        clock.lock().unwrap().push((TaskId::A, ns));
        iter += 1;
        notify.notify_waiters();
    }
}

/// Consumer used in asynchroneous function
async fn consumer_task(notify : Arc<Notify>, clock_iter : Arc<AtomicUsize>, clock : Arc<Mutex<Vec<(TaskId, u128)>>>) {
    let mut iter : i32 = 0;
    while iter < LOOP_CNT {
        notify.notified().await;
        let ns: u128 = SystemTime::now().duration_since(UNIX_EPOCH).expect("").as_nanos();
        clock_iter.fetch_add(1, Ordering::AcqRel);
        clock.lock().unwrap().push((TaskId::B, ns));
        iter += 1;
        notify.notify_waiters();
    }
}

async fn old_fn() -> std::io::Result<()> {
    let mut array : Vec<(crate::TaskId, u128)> = Vec::new();
    array.reserve(2048);

    //For async and threading
    let clock_vec : Arc<Mutex<Vec<(crate::TaskId,u128)>>> = Arc::new(Mutex::from(array));
    let clock_iter = Arc::new(AtomicUsize::new(0));
    let notify1 = Arc::new(Notify::new());
    let notify2 = notify1.clone();

    let tasks = (
        tokio::task::spawn( producer_task(notify1,Arc::clone(&clock_iter), Arc::clone(&clock_vec))),
        tokio::task::spawn( consumer_task(notify2,Arc::clone(&clock_iter), Arc::clone(&clock_vec)))
    ).join().await;
    if tasks.0.is_err() || tasks.1.is_err()
    { std::io::Error::new(std::io::ErrorKind::Interrupted,"fail"); }


    let mut str_consume = String::from("Consumer:\n");
    let mut str_produce = String::from("Producer:\n");
    for item in (*clock_vec.lock().unwrap()).iter() {
        match item.0 {
            TaskId::A => {
                str_produce.push_str(&item.1.to_string());
                str_produce.push_str("\n");
            },
            TaskId::B => {
                str_consume.push_str(&item.1.to_string());
                str_consume.push_str("\n");
            },
        }
    }
    let mut file_prod: File = File::create("producer.csv")?;
    let mut file_cons: File = File::create("consumer.csv")?;
    file_prod.write_all(str_produce.as_bytes())?;
    file_cons.write_all(str_consume.as_bytes())?;
    Ok(())
}