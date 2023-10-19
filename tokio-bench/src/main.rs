/// Implement an asynchroneous (and thread) task benchchmark for tokio runtime
use tokio;
use tokio::sync::Notify;
use futures_concurrency::future::Join;
use shared::{AppSettings, TaskId};
use std::{
    sync::{ Arc, mpsc::{ channel, Sender, Receiver } },
    time::{ Instant, Duration },
    thread,
};

/// Small function to print iteration index in the bash
pub fn print_iter(iter : &usize, id : &TaskId) {
    let name_task: &str = id.nameof();
    println!("{name_task} - {iter}");
}

/// Producer used in asynchroneous function
async fn task_generic(notify : Arc<Notify>, trigger : Option<Arc<Notify>>, id : TaskId, settings : AppSettings) -> Vec<(TaskId, u128)> {
    //Init
    let mut iter : usize = 0;
    let mut clock : Vec<(crate::TaskId, u128)> = Vec::with_capacity(settings.limit);
    let instant: Instant  = Instant::now();
    let is_first: bool = trigger.is_some();

    // Loop
    while iter < settings.limit {
        if iter == 0 && is_first {
            trigger.as_ref().unwrap().notified().await; }
        else {
            notify.notified().await; }
        clock.push((id, instant.elapsed().as_nanos()));
        iter += 1;

        if settings.display_iter {
            print_iter(&iter, &id)
        }
        notify.notify_waiters();
    }
    clock
}

/// Producer or consumer used in synchrone function
fn thread_generic(tx : Sender<()>, rx : Receiver<()>, trigger : Option<Receiver<()>>, id : TaskId, settings : AppSettings) -> Vec<(TaskId,u128)> {
    //Init
    let mut iter : usize = 0;
    let mut clock : Vec<(crate::TaskId, u128)> = Vec::with_capacity(settings.limit);
    let instant: Instant = Instant::now();
    let is_first: bool = trigger.is_some();

    //Loop
    while iter < settings.limit {
        if iter == 0 && is_first {
            _ = trigger.as_ref().unwrap().recv().unwrap(); }
        else {
            _ = rx.recv().unwrap(); }
        clock.push((id, instant.elapsed().as_nanos()));
        iter += 1;

        if settings.display_iter {
            print_iter(&iter, &id)
        }
        _ = tx.send(());
    }

    clock
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    // Read app arguments
    let settings : AppSettings = shared::read_app_argument();

    // Try to set thread priority
    thread_priority::set_current_thread_priority(thread_priority::ThreadPriority::Max).unwrap_or(());

    //For async and threading
    let trigger = Arc::new(Notify::new());
    let notify = Arc::new(Notify::new());

    //Tokio tasks
    let tasks = (
        tokio::task::spawn( task_generic(notify.clone(), Some(trigger.clone()), TaskId::A, settings.clone())),
        tokio::task::spawn( task_generic(notify.clone(), None, TaskId::B, settings.clone()))
    ).join();
    std::thread::sleep(Duration::from_micros(shared::LAZY_START));
    trigger.notify_one();
    let results: (Result<Vec<(TaskId, u128)>, tokio::task::JoinError>, Result<Vec<(TaskId, u128)>, tokio::task::JoinError>) = tasks.await;
    if settings.output_file.is_empty() {
        shared::format_str(results.0.as_ref().unwrap(), results.1.as_ref().unwrap() ,"tokio_diff.csv")?; }
    else {
        shared::format_str(results.0.as_ref().unwrap(), results.1.as_ref().unwrap() ,settings.output_file.as_str())?; }

    //For threading
    let (tx1, rx1) : (Sender<()>, Receiver<()>) = channel();
    let (tx2, rx2) : (Sender<()>, Receiver<()>) = channel();
    let (tx_trigger,rx_trigger) = channel();
    let settings_cpy1 = settings.clone();
    let settings_cpy2 = settings.clone();

    let threads = (
        thread::spawn( move || thread_generic(tx1, rx2, Some(rx_trigger) ,TaskId::A, settings_cpy1)),
        thread::spawn( move || thread_generic(tx2, rx1, None, TaskId::B, settings_cpy2)),
    );
    std::thread::sleep(Duration::from_micros(shared::LAZY_START));
    _ = tx_trigger.send(());
    if settings.output_file.is_empty() {
        shared::format_str(&threads.0.join().unwrap(), &threads.1.join().unwrap() ,"tokio_diff_thread.csv")?;
    }
    else {
        let mut title = String::from("thread_");
        title.push_str(settings.output_file.as_str());
        shared::format_str(&threads.0.join().unwrap(), &threads.1.join().unwrap() , title.as_str())?;
    }

    Ok(())
}


// ============================== DEPRECATED ==================================


// Producer used in asynchroneous function
// async fn producer_task(notify : Arc<Notify>, clock_iter : Arc<AtomicUsize>, clock : Arc<Mutex<Vec<(TaskId, u128)>>> ) {
//     let mut iter : usize = 0;
//     while iter < DEFAULT_LOOP_CNT {
//         if iter != 0 { notify.notified().await; }
//         else { std::thread::sleep(Duration::from_micros(LAZY_START)); }
//         let ns: u128 = SystemTime::now().duration_since(UNIX_EPOCH).expect("").as_nanos();
//         clock_iter.fetch_add(1, Ordering::AcqRel);
//         clock.lock().unwrap().push((TaskId::A, ns));
//         iter += 1;
//         notify.notify_waiters();
//     }
// }

// Consumer used in asynchroneous function
// async fn consumer_task(notify : Arc<Notify>, clock_iter : Arc<AtomicUsize>, clock : Arc<Mutex<Vec<(TaskId, u128)>>>) {
//     let mut iter : usize = 0;
//     while iter < DEFAULT_LOOP_CNT {
//         notify.notified().await;
//         let ns: u128 = SystemTime::now().duration_since(UNIX_EPOCH).expect("").as_nanos();
//         clock_iter.fetch_add(1, Ordering::AcqRel);
//         clock.lock().unwrap().push((TaskId::B, ns));
//         iter += 1;
//         notify.notify_waiters();
//     }
// }

// async fn old_fn() -> std::io::Result<()> {
//     let mut array : Vec<(crate::TaskId, u128)> = Vec::new();
//     array.reserve(2048);

//     //For async and threading
//     let clock_vec : Arc<Mutex<Vec<(crate::TaskId,u128)>>> = Arc::new(Mutex::from(array));
//     let clock_iter = Arc::new(AtomicUsize::new(0));
//     let notify1 = Arc::new(Notify::new());
//     let notify2 = notify1.clone();

//     let tasks = (
//         tokio::task::spawn( producer_task(notify1,Arc::clone(&clock_iter), Arc::clone(&clock_vec))),
//         tokio::task::spawn( consumer_task(notify2,Arc::clone(&clock_iter), Arc::clone(&clock_vec)))
//     ).join().await;
//     if tasks.0.is_err() || tasks.1.is_err()
//     { std::io::Error::new(std::io::ErrorKind::Interrupted,"fail"); }


//     let mut str_consume = String::from("Consumer:\n");
//     let mut str_produce = String::from("Producer:\n");
//     for item in (*clock_vec.lock().unwrap()).iter() {
//         match item.0 {
//             TaskId::A => {
//                 str_produce.push_str(&item.1.to_string());
//                 str_produce.push_str("\n");
//             },
//             TaskId::B => {
//                 str_consume.push_str(&item.1.to_string());
//                 str_consume.push_str("\n");
//             },
//             _ => (),
//         }
//     }
//     let mut file_prod: File = File::create("producer.csv")?;
//     let mut file_cons: File = File::create("consumer.csv")?;
//     file_prod.write_all(str_produce.as_bytes())?;
//     file_cons.write_all(str_consume.as_bytes())?;
//     Ok(())
// }