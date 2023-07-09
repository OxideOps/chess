use std::{
    sync::{
        mpsc::{self, Receiver, Sender},
        Arc, Mutex,
    },
    thread,
    time::Duration,
};

pub struct Timer {
    white: Arc<Mutex<Duration>>,
    black: Arc<Mutex<Duration>>,
    active: Arc<Mutex<&'static str>>,
    command_sender: Sender<Command>,
}

enum Command {
    Switch,
    Stop,
}

impl Timer {
    fn new() -> Self {
        let (tx, _) = mpsc::channel();
        let white = Arc::new(Mutex::new(Duration::from_secs(600))); // 10 minutes
        let black = Arc::new(Mutex::new(Duration::from_secs(600))); // 10 minutes
        let active = Arc::new(Mutex::new("white"));

        Timer {
            white,
            black,
            active,
            command_sender: tx,
        }
    }

    fn start(&mut self) {
        let (tx, rx) = mpsc::channel();
        self.command_sender = tx;

        let white_clone = Arc::clone(&self.white);
        let black_clone = Arc::clone(&self.black);
        let active_clone = Arc::clone(&self.active);

        thread::spawn(move || Self::timer_thread(white_clone, black_clone, active_clone, rx));
    }

    fn next(&self) {
        self.command_sender.send(Command::Switch).unwrap();
    }

    fn timer_thread(
        white: Arc<Mutex<Duration>>,
        black: Arc<Mutex<Duration>>,
        active: Arc<Mutex<&'static str>>,
        command_receiver: Receiver<Command>,
    ) {
        loop {
            // Check if we should stop or switch
            match command_receiver.try_recv() {
                Ok(Command::Switch) => {
                    let mut active_guard = active.lock().unwrap();
                    *active_guard = if *active_guard == "white" {
                        "black"
                    } else {
                        "white"
                    };
                }
                Ok(Command::Stop) | Err(_) => {}
            }

            // Decrement the active timer
            {
                let active_timer = if *active.lock().unwrap() == "white" {
                    &white
                } else {
                    &black
                };
                let mut active_timer_guard = active_timer.lock().unwrap();
                if *active_timer_guard == Duration::from_secs(0) {
                    println!(
                        "{} ran out of time",
                        if Arc::ptr_eq(&active_timer, &white) {
                            "white"
                        } else {
                            "black"
                        }
                    );
                    return;
                }
                *active_timer_guard -= Duration::from_secs(1);
            }

            thread::sleep(Duration::from_secs(1));
        }
    }
}
