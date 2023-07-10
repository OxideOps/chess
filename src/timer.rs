use std::{
    sync::{
        mpsc::{self, Receiver, Sender},
        Arc, Mutex, MutexGuard,
    },
    thread,
    time::Duration,
};

use crate::pieces::Player;

pub struct Timer {
    white: Arc<Mutex<Duration>>,
    black: Arc<Mutex<Duration>>,
    active: Arc<Mutex<&'static str>>,
    command_sender: Option<Arc<Mutex<Sender<Command>>>>,
}

enum Command {
    Switch,
    Stop,
}

impl Default for Timer {
    fn default() -> Self {
        Timer::with_duration(Duration::from_secs(60))
    }
}

impl Timer {
    fn with_duration(duration: Duration) -> Self {
        Timer {
            white: Arc::new(Mutex::new(duration)),
            black: Arc::new(Mutex::new(duration)),
            active: Arc::new(Mutex::new("white")),
            command_sender: None,
        }
    }

    pub fn get_duration(&self, player: Player) -> MutexGuard<Duration> {
        match player {
            Player::White => self.white.lock().unwrap(),
            Player::Black => self.white.lock().unwrap(),
        }
    }

    pub fn start(&mut self) {
        let (tx, rx) = mpsc::channel();
        let command_sender = Arc::new(Mutex::new(tx));
        self.command_sender = Some(command_sender.clone());

        let white_clone = Arc::clone(&self.white);
        let black_clone = Arc::clone(&self.black);
        let active_clone = Arc::clone(&self.active);

        thread::spawn(move || Self::timer_thread(white_clone, black_clone, active_clone, rx));
    }

    pub fn next(&self) {
        self.command_sender
            .as_ref()
            .unwrap()
            .lock()
            .unwrap()
            .send(Command::Switch)
            .unwrap()
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
                Ok(Command::Stop) | Err(..) => {}
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
