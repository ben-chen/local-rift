//! One bounded background opponent step; no rendering or shared mutable Game.
use local_rift::game::Game;
use std::sync::mpsc::{self, Receiver, SyncSender, TryRecvError, TrySendError};

type StepResult = Result<Game, String>;
struct Job {
    revision: u64,
    game: Game,
}
struct Completion {
    revision: u64,
    result: StepResult,
}
struct Worker {
    jobs: SyncSender<Job>,
    results: Receiver<Completion>,
}

type Stepper = Box<dyn FnMut(Game) -> StepResult + Send>;

#[derive(Default)]
pub(crate) struct AiWorker {
    revision: u64,
    worker: Option<Worker>,
    pending: Option<u64>,
    ready: Option<Game>,
    failure: Option<String>,
    unreported: Option<String>,
    stepper: Option<Stepper>,
}
impl AiWorker {
    /// Reset invalidates the result, not the worker. In-flight stale work may
    /// finish, but cannot alter the replacement match or create more workers.
    pub fn invalidate(&mut self) {
        self.revision = self.revision.wrapping_add(1);
        self.ready = None;
        self.failure = None;
        self.unreported = None;
    }
    pub fn failure(&self) -> Option<&str> {
        self.failure.as_deref()
    }
    pub fn busy(&self) -> bool {
        self.pending.is_some()
    }
    pub fn can_start(&self) -> bool {
        self.pending.is_none() && self.ready.is_none() && self.failure.is_none()
    }
    pub fn retry(&mut self) {
        self.failure = None;
        self.unreported = None;
    }
    fn fail(&mut self, message: String) {
        self.failure = Some(message.clone());
        self.unreported = Some(message);
    }
    fn spawn(&mut self) -> Result<(), String> {
        let (jobs, requests) = mpsc::sync_channel::<Job>(1);
        let (completed, results) = mpsc::sync_channel(1);
        let mut stepper = self.stepper.take().unwrap_or_else(|| {
            Box::new(|mut game| {
                game.step_ai()?;
                Ok(game)
            })
        });
        // The handle is deliberately detached: neither polling nor Drop joins
        // an AI search. Closing the channels makes this loop exit after its job.
        std::thread::Builder::new().name("local-rift-opponent".into()).spawn(move || {
            while let Ok(job) = requests.recv() {
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| stepper(job.game)))
                    .unwrap_or_else(|_| Err("Opponent calculation stopped unexpectedly. Retry the move or choose a new match.".into()));
                if completed.send(Completion { revision: job.revision, result }).is_err() { break; }
            }
        }).map_err(|_| "Could not start the opponent calculation. Retry the move or choose a new match.".to_owned())?;
        self.worker = Some(Worker { jobs, results });
        Ok(())
    }
    pub fn request(&mut self, game: Game) {
        if !self.can_start() {
            return;
        }
        if self.worker.is_none()
            && let Err(error) = self.spawn()
        {
            self.fail(error);
            return;
        }
        let job = Job {
            revision: self.revision,
            game,
        };
        match self.worker.as_ref().unwrap().jobs.try_send(job) {
            Ok(()) => self.pending = Some(self.revision),
            Err(TrySendError::Disconnected(_)) => {
                self.worker = None;
                self.fail(
                    "Opponent calculation is unavailable. Retry the move or choose a new match."
                        .into(),
                );
            }
            Err(TrySendError::Full(_)) => {
                // Preserve the worker if its queue is unexpectedly occupied;
                // replacing it could create concurrent searches.
                self.pending = Some(self.revision);
                self.fail("Opponent calculation is already pending. Wait for it to finish before retrying.".into());
            }
        }
    }
    /// Always nonblocking, including while a card inspector or menu is open.
    pub fn poll(&mut self) {
        let Some(pending_revision) = self.pending else {
            return;
        };
        let result = self.worker.as_ref().unwrap().results.try_recv();
        match result {
            Ok(completion) => {
                self.pending = None;
                if completion.revision == self.revision {
                    match completion.result {
                        Ok(game) => self.ready = Some(game),
                        Err(error) => self.fail(error),
                    }
                }
            }
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => {
                self.pending = None;
                self.worker = None;
                if pending_revision == self.revision {
                    self.fail(
                        "Opponent calculation stopped. Retry the move or choose a new match."
                            .into(),
                    );
                }
            }
        }
    }
    /// View-only navigation and modals hold the valid result without a retry.
    pub fn take_completed(&mut self, allowed: bool) -> Option<StepResult> {
        if !allowed {
            return None;
        }
        if let Some(error) = self.unreported.take() {
            return Some(Err(error));
        }
        self.ready.take().map(Ok)
    }
    pub(crate) fn with_stepper(stepper: impl FnMut(Game) -> StepResult + Send + 'static) -> Self {
        Self {
            stepper: Some(Box::new(stepper)),
            ..Self::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use local_rift::game::{Action, Difficulty};
    use std::time::{Duration, Instant};
    fn game(seed: u64) -> Game {
        let mut game = Game::new_for_agents(3, 3, seed, Difficulty::Easy);
        game.start_match().unwrap();
        game.apply(Action::EndTurn).unwrap();
        game
    }
    fn wait(worker: &mut AiWorker) {
        let limit = Instant::now() + Duration::from_secs(5);
        while worker.busy() {
            worker.poll();
            assert!(Instant::now() < limit, "worker did not complete");
            std::thread::sleep(Duration::from_millis(1));
        }
    }
    #[test]
    fn pending_search_never_blocks_poll_and_only_one_job_is_accepted() {
        let (started, running) = mpsc::channel();
        let (release, gate) = mpsc::channel();
        let mut worker = AiWorker::with_stepper(move |game| {
            started.send(()).unwrap();
            gate.recv().unwrap();
            Ok(game)
        });
        worker.request(game(1));
        running.recv_timeout(Duration::from_secs(1)).unwrap();
        for _ in 0..100 {
            worker.poll();
            assert!(worker.busy());
            assert!(worker.take_completed(true).is_none());
            assert!(!worker.can_start());
        }
        worker.request(game(2)); // No queued replacement or extra worker.
        release.send(()).unwrap();
        wait(&mut worker);
        assert!(worker.take_completed(true).unwrap().is_ok());
        assert!(running.try_recv().is_err());
    }
    #[test]
    fn reset_discards_old_success_while_fresh_human_actions_remain_independent() {
        let (release, gate) = mpsc::channel();
        let mut worker = AiWorker::with_stepper(move |mut game| {
            gate.recv().unwrap();
            game.step_ai()?;
            Ok(game)
        });
        worker.request(game(2));
        worker.invalidate();
        let mut fresh = Game::new_for_agents(0, 1, 3, Difficulty::Easy);
        fresh.start_match().unwrap();
        fresh.apply(Action::EndTurn).unwrap();
        let snapshot = format!("{fresh:?}");
        assert!(worker.busy());
        release.send(()).unwrap();
        wait(&mut worker);
        assert!(worker.take_completed(true).is_none());
        assert_eq!(format!("{fresh:?}"), snapshot);
        assert!(worker.can_start());
    }
    #[test]
    fn modal_and_navigation_hold_a_valid_result_until_reopened() {
        let original = game(4);
        let mut expected = original.clone();
        expected.step_ai().unwrap();
        let mut worker = AiWorker::default();
        worker.request(original);
        wait(&mut worker);
        for _ in 0..20 {
            assert!(worker.take_completed(false).is_none());
            assert!(!worker.can_start());
        }
        let actual = worker.take_completed(true).unwrap().unwrap();
        assert_eq!(format!("{actual:?}"), format!("{expected:?}"));
        assert!(worker.take_completed(true).is_none());
    }
    #[test]
    fn a_match_mutation_also_invalidates_an_already_completed_result() {
        let mut worker = AiWorker::default();
        worker.request(game(5));
        wait(&mut worker);
        worker.invalidate();
        assert!(worker.take_completed(true).is_none());
        assert!(worker.can_start());
    }
    #[test]
    fn partial_error_and_panic_never_commit_and_require_explicit_retry() {
        for panic in [false, true] {
            let mut worker = AiWorker::with_stepper(move |mut game| {
                game.players[0].score = 7;
                if panic {
                    panic!("injected search failure");
                }
                Err("injected engine failure".into())
            });
            worker.request(game(6));
            wait(&mut worker);
            assert!(worker.take_completed(true).unwrap().is_err());
            assert!(worker.failure().is_some());
            for _ in 0..100 {
                worker.poll();
                assert!(!worker.can_start());
                assert!(worker.take_completed(true).is_none());
            }
            worker.retry();
            assert!(worker.can_start());
        }
    }
    #[test]
    fn one_worker_step_preserves_full_rng_and_exact_game_semantics() {
        let mut live = game(7);
        let mut worker = AiWorker::default();
        for _ in 0..6 {
            let mut expected = live.clone();
            expected.step_ai().unwrap();
            worker.request(live);
            wait(&mut worker);
            live = worker.take_completed(true).unwrap().unwrap();
            worker.invalidate();
            assert_eq!(format!("{live:?}"), format!("{expected:?}"));
        }
    }
    #[test]
    fn stale_failure_does_not_disable_the_replacement_match() {
        let (release, gate) = mpsc::channel();
        let mut worker = AiWorker::with_stepper(move |_| {
            gate.recv().unwrap();
            Err("old match failed".into())
        });
        worker.request(game(9));
        worker.invalidate();
        release.send(()).unwrap();
        wait(&mut worker);
        assert!(worker.take_completed(true).is_none());
        assert!(worker.failure().is_none());
        assert!(worker.can_start());
    }
    #[test]
    fn disconnected_worker_reports_once_and_can_be_recreated_explicitly() {
        let (jobs, requests) = mpsc::sync_channel(1);
        let (completed, results) = mpsc::sync_channel(1);
        drop(requests);
        drop(completed);
        let mut worker = AiWorker {
            worker: Some(Worker { jobs, results }),
            pending: Some(0),
            ..AiWorker::default()
        };
        worker.poll();
        assert!(!worker.busy());
        assert!(worker.take_completed(false).is_none());
        assert!(worker.take_completed(true).unwrap().is_err());
        assert!(worker.take_completed(true).is_none());
        assert!(!worker.can_start());
        worker.retry();
        worker.request(game(10));
        wait(&mut worker);
        assert!(worker.take_completed(true).unwrap().is_ok());
    }
    #[test]
    fn dropping_a_busy_worker_does_not_join_the_search() {
        let (started, running) = mpsc::channel();
        let (release, gate) = mpsc::channel();
        let mut worker = AiWorker::with_stepper(move |game| {
            started.send(()).unwrap();
            gate.recv().unwrap();
            Ok(game)
        });
        worker.request(game(8));
        running.recv_timeout(Duration::from_secs(1)).unwrap();
        drop(worker);
        release.send(()).unwrap();
    }
}
