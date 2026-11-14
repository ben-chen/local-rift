//! Shared protocol operations for authored rules scenarios.
use super::*;

pub(super) fn request(s: &mut Session, command: &str, action: Value) -> Value {
    s.handle(json!({"command":command,"revision":s.revision(),"player":s.game.priority(),"action":action}))
}

#[track_caller]
pub(super) fn apply(s: &mut Session, action: Action) {
    let result = request(s, "apply", action_to_json(&action));
    assert_eq!(result["ok"], true, "{result}");
}

pub(super) fn views(s: &Session) -> [Value; 2] {
    [s.observe(0).unwrap(), s.observe(1).unwrap()]
}
