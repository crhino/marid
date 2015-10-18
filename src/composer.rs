use crossbeam;
use traits::{Runner, Signal, Receiver, Sender};
use chan;
use std::thread;
use std::sync::{Arc, Mutex};

pub struct Composer<R: Runner> {
    runners: Vec<R>,
    state: State,
    error: Arc<Mutex<Option<R::Error>>>,
}

enum State {
    Init,
    SetupDone,
}

impl<R: Runner> Composer<R> {
    pub fn new(runners: Vec<R>) -> Composer<R> {
        Composer{
            runners: runners,
            state: State::Init,
            error: Arc::new(Mutex::new(None)),
        }
    }
}

impl<R: Runner + Send> Runner for Composer<R> {
    type Error = R::Error;
    fn run(mut self, signals: Receiver<Signal>) -> Result<(), Self::Error> {
        match self.state {
            State::Init => try!(self.setup()),
            _ => {},
        }

        let mut runners_vec = vec!();
        let mut sender_vec = vec!();
        for r in self.runners.into_iter() {
            let (sn, rc) = chan::sync(1024);
            runners_vec.push((r, rc));
            sender_vec.push(sn);
        }

        let (stop_sn, stop_rc) = chan::sync(0);
        let signaling = signaling_thread(signals, sender_vec, stop_rc);
        let error = self.error.clone();

        crossbeam::scope(|scope| {
            for (r, rc) in runners_vec.into_iter() {
                scope.spawn(|| {
                    match r.run(rc) {
                        Ok(()) => {},
                        Err(e) => {
                            let mut guard = error.lock().unwrap();
                            *guard = Some(e);
                        },
                    }
                });
            }
        });

        stop_sn.send(true);
        signaling.join().unwrap();
        match self.error.lock().unwrap().take() {
            Some(e) => Err(e),
            None => Ok(()),
        }
    }

    fn setup(&mut self) -> Result<(), Self::Error> {
        for r in self.runners.iter_mut() {
            try!(r.setup());
        }
        self.state = State::SetupDone;
        Ok(())
    }
}

fn signaling_thread(signals: Receiver<Signal>,
                    senders: Vec<Sender<Signal>>,
                    quit: Receiver<bool>) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        loop {
            chan_select! {
                signals.recv() -> res => {
                    let sig = match res {
                        Some(s) => s,
                        None => continue,
                    };

                    for sn in senders.iter() {
                        sn.send(sig);
                    }
                },
                quit.recv() => {
                    return
                }
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use test_helpers::{TestRunner};
    use {Composer, Runner, Signal};
    use chan;
    use std::thread;

    #[test]
    fn test_composer_runner() {
        let (sn, rc) = chan::sync(2);

        let runner1 = TestRunner::new(1, sn.clone());
        let runner2 = TestRunner::new(2, sn);

        let (sig_send, signals) = chan::async();

        let mut composer = Composer::new(vec!(runner1, runner2));
        let res = composer.setup();
        assert!(res.is_ok());

        thread::spawn(move || {
            sig_send.send(Signal::INT);
        });

        let res = composer.run(signals);
        assert!(res.is_ok());

        assert!(rc.recv().expect("Did not recv"));
        assert!(rc.recv().expect("Did not recv"));
    }

    #[test]
    fn test_composer_error() {
        let (sn, rc) = chan::sync(2);

        let runner1 = TestRunner::new(1, sn.clone());
        let runner2 = TestRunner::new(2, sn);

        let (sig_send, signals) = chan::async();

        let mut composer = Composer::new(vec!(runner1, runner2));
        let res = composer.setup();
        assert!(res.is_ok());

        thread::spawn(move || {
            sig_send.send(Signal::HUP);
        });

        let res = composer.run(signals);
        assert!(res.is_err());

        assert!(!rc.recv().expect("Did not recv"));
        assert!(!rc.recv().expect("Did not recv"));
    }

    #[test]
    fn test_composer_run_no_setup() {
        let (sn, rc) = chan::sync(2);

        let runner1 = TestRunner::new(1, sn.clone());
        let runner2 = TestRunner::new(2, sn);

        let (sig_send, signals) = chan::async();

        let composer = Composer::new(vec!(runner1, runner2));
        thread::spawn(move || {
            sig_send.send(Signal::INT);
        });

        let res = composer.run(signals);
        assert!(res.is_ok());

        assert!(rc.recv().expect("Did not recv"));
        assert!(rc.recv().expect("Did not recv"));
    }
}
