use local_rift::api::{ResetOptions, Session};
use serde_json::json;

#[test]
fn strategic_difficulty_is_accepted_by_the_json_protocol() {
    let mut session = Session::new(ResetOptions::default()).unwrap();
    let response = session
        .handle(json!({"command":"reset", "revision":0, "difficulty":"strategic", "seed":42}));
    assert_eq!(response["ok"], true, "{response}");
    session.start(1).unwrap();
    let player = session.observe(0).unwrap()["priority"].as_u64().unwrap() as usize;
    session.step_ai(player, 2).unwrap();
    assert_eq!(session.revision(), 3);
}
