//! Native responsiveness checks with a deliberately blocked opponent worker.
//! The injected step still executes one canonical step_ai after its gate opens.
use super::*;
use crate::ai_worker::AiWorker;
use std::sync::{Arc, Mutex, mpsc};
use std::time::{Duration, Instant};

#[derive(Default)]
struct Progress {
    stage: usize,
    since: Option<Instant>,
    release: Option<mpsc::Sender<()>>,
    started: Option<mpsc::Receiver<()>>,
    before: String,
    expected: Arc<Mutex<Option<String>>>,
    attempts: Arc<std::sync::atomic::AtomicUsize>,
}
thread_local! { static PROGRESS: std::cell::RefCell<Progress> = std::cell::RefCell::new(Progress::default()); }
fn position(app: &mut App) {
    expansion_fixture(app, 412);
    app.game = Game::new_for_agents(3, 3, 270027, Difficulty::Easy);
    app.game.start_match().unwrap();
    app.game.apply(Action::EndTurn).unwrap();
    app.ai_worker.invalidate();
    app.fields = app.game.battlefields;
    app.inspect = None;
    app.lesson = None;
    app.confirm_restart = false;
    app.show_journal = false;
    app.screen = Screen::Practice;
    app.in_match = true;
}
fn blocked(app: &mut App, progress: &mut Progress) {
    let (release, gate) = mpsc::channel();
    let (started, receiver) = mpsc::channel();
    let expected = Arc::new(Mutex::new(None));
    let result = expected.clone();
    app.ai_worker = AiWorker::with_stepper(move |mut game| {
        started.send(()).unwrap();
        gate.recv().unwrap();
        game.step_ai()?;
        *result.lock().unwrap() = Some(format!("{game:?}"));
        Ok(game)
    });
    progress.release = Some(release);
    progress.started = Some(receiver);
    progress.expected = expected;
    progress.before = format!("{:?}", app.game);
    app.next_ai = 0.0;
    app.update();
    app.next_ai = f64::INFINITY;
    assert!(app.ai_worker.busy());
}
fn unchanged(app: &App, progress: &Progress) {
    assert_eq!(format!("{:?}", app.game), progress.before);
}
fn completed(app: &App, progress: &Progress) {
    assert_eq!(
        Some(format!("{:?}", app.game)),
        *progress.expected.lock().unwrap()
    );
}
pub(crate) fn run_frame(app: &mut App, frame: u32) -> bool {
    eprintln!("AI UI frame {frame}");
    app.next_ai = f64::INFINITY;
    app.ui.input_override = Some((vec2(-100.0, -100.0), false, false));
    app.ui.pointer_override = Some((false, false));
    app.ui.wheel_override = Some(0.0);
    PROGRESS.with_borrow_mut(|p| {
        let since = p.since.get_or_insert_with(Instant::now);
        assert!(since.elapsed() < Duration::from_secs(20), "AI UI stage {} timed out", p.stage);
        match p.stage {
            0 => { position(app); blocked(app, p); }
            1 => {
                if p.started.as_ref().unwrap().try_recv().is_err() { return false; }
                unchanged(app, p);
                capture(app, "RIFT_SMOKE_AI_PENDING_SCREENSHOT");
                app.ui.input_override = Some((vec2(315.0, 760.0), false, true));
            }
            2 => {
                assert!(app.inspect.is_some());
                assert!(app.ai_worker.busy());
                unchanged(app, p);
                capture(app, "RIFT_SMOKE_AI_INSPECT_SCREENSHOT");
                click(app, 960.0, 704.0);
            }
            3 => { assert!(app.inspect.is_none()); click(app, 100.0, 322.0); }
            4 => {
                assert!(app.screen == Screen::Settings);
                assert!(app.ai_worker.busy());
                unchanged(app, p);
                p.release.take().unwrap().send(()).unwrap();
            }
            5 => {
                if app.ai_worker.busy() { return false; }
                assert!(app.screen == Screen::Settings);
                unchanged(app, p);
                assert!(!app.ai_worker.can_start());
                capture(app, "RIFT_SMOKE_AI_SETTINGS_HOLD_SCREENSHOT");
                click(app, 100.0, 270.0);
            }
            6 => {
                assert!(app.screen == Screen::Guide);
                unchanged(app, p);
                capture(app, "RIFT_SMOKE_AI_GUIDE_HOLD_SCREENSHOT");
                click(app, 100.0, 166.0);
            }
            7 => {
                completed(app, p);
                assert!(!app.ai_worker.busy());
                capture(app, "RIFT_SMOKE_AI_COMMITTED_SCREENSHOT");
                position(app); blocked(app, p);
            }
            8 => {
                if p.started.as_ref().unwrap().try_recv().is_err() { return false; }
                click(app, 1320.0, 46.0);
            }
            9 => {
                assert!(app.confirm_restart);
                unchanged(app, p);
                capture(app, "RIFT_SMOKE_AI_RESTART_PENDING_SCREENSHOT");
                click(app, 850.0, 501.0);
            }
            10 => {
                assert!(!app.in_match);
                assert!(app.ai_worker.busy());
                click(app, 1320.0, 46.0);
            }
            11 => {
                assert!(app.in_match && app.game.phase == Phase::Mulligan);
                assert!(app.ai_worker.busy());
                click(app, 1285.0, 514.0);
            }
            12 => {
                assert!(app.game.phase == Phase::Playing);
                assert_eq!(app.game.priority(), 0);
                assert!(app.ai_worker.busy());
                capture(app, "RIFT_SMOKE_AI_FRESH_HUMAN_TURN_SCREENSHOT");
                click(app, 1285.0, 596.0);
            }
            13 => {
                assert_eq!(app.game.priority(), 1);
                assert!(app.ai_worker.busy());
                p.before = format!("{:?}", app.game);
                p.release.take().unwrap().send(()).unwrap();
            }
            14 => {
                if app.ai_worker.busy() { return false; }
                unchanged(app, p);
                assert!(app.ai_worker.can_start());
                capture(app, "RIFT_SMOKE_AI_STALE_DISCARDED_SCREENSHOT");
                let attempts = p.attempts.clone();
                let expected = Arc::new(Mutex::new(None));
                let result = expected.clone();
                p.expected = expected;
                app.ai_worker = AiWorker::with_stepper(move |mut game| {
                    if attempts.fetch_add(1, std::sync::atomic::Ordering::SeqCst) == 0 {
                        game.players[0].score = 7;
                        return Err("Injected opponent failure".into());
                    }
                    game.step_ai()?;
                    *result.lock().unwrap() = Some(format!("{game:?}"));
                    Ok(game)
                });
                app.next_ai = 0.0; app.update(); app.next_ai = f64::INFINITY;
            }
            15 => {
                if app.ai_worker.busy() { return false; }
                assert!(app.ai_worker.failure().is_some());
                unchanged(app, p);
                assert_eq!(p.attempts.load(std::sync::atomic::Ordering::SeqCst), 1);
                capture(app, "RIFT_SMOKE_AI_FAILED_RETRY_SCREENSHOT");
            }
            16..=18 => {
                app.next_ai = 0.0; app.update(); app.next_ai = f64::INFINITY;
                unchanged(app, p);
                assert_eq!(p.attempts.load(std::sync::atomic::Ordering::SeqCst), 1);
                assert!(!app.ai_worker.can_start());
            }
            19 => { click(app, 1285.0, 532.0); }
            20 => {
                // Full smoke disables automatic scheduling, so explicitly run
                // the normal update once after the actual Retry control click.
                assert!(app.ai_worker.failure().is_none());
                app.next_ai = 0.0; app.update(); app.next_ai = f64::INFINITY;
            }
            21 => {
                if app.ai_worker.busy() { return false; }
                completed(app, p);
                assert_eq!(p.attempts.load(std::sync::atomic::Ordering::SeqCst), 2);
                capture(app, "RIFT_SMOKE_AI_RETRY_FINISHED_SCREENSHOT");
                println!("PASS: 22 AI UI stages; inspection/navigation render while blocked, completed results wait, reset discards stale work, fresh human actions work, failure retries only on click.");
                return true;
            }
            _ => unreachable!(),
        }
        eprintln!("AI UI stage {} passed", p.stage);
        p.stage += 1;
        p.since = Some(Instant::now());
        false
    })
}
