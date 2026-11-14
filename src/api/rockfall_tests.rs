use super::test_support::{apply, request, views};
use super::*;
use crate::battlefields::ROCKFALL_PATH_ID;

#[test]
fn rockfall_api_catalog_and_legality_use_the_physical_battlefield() {
    assert_eq!(
        catalog()["battlefields"][ROCKFALL_PATH_ID]["official_text"],
        "Units can't be played here."
    );
    for actor in 0..2 {
        let mut s = Session::new(ResetOptions::default()).unwrap();
        s.game = crate::game::test_support::position(actor, 0, 17002);
        // A third physical field catches accidental lane == definition-ID checks.
        assert_eq!(s.game.add_battlefield(ROCKFALL_PATH_ID), 2);
        s.game.battlefield_owner[2] = Some(actor);
        s.game.players[actor].hand = vec![71];
        let before = views(&s);
        let revision = s.revision();
        let action = action_to_json(&Action::PlayCardWithTargets {
            hand_index: 0,
            lane: Some(2),
            targets: vec![],
        });
        for command in ["validate", "apply"] {
            let result = request(&mut s, command, action.clone());
            assert_eq!(result["ok"], false);
            assert!(result.to_string().contains("Rockfall Path"));
            assert_eq!(views(&s), before);
            assert_eq!(s.revision(), revision);
        }
        let suggestions = s.suggestions(actor).unwrap();
        for choice in suggestions {
            assert_ne!(action_to_json(&choice.action), action);
        }
        apply(
            &mut s,
            Action::PlayCardWithTargets {
                hand_index: 0,
                lane: None,
                targets: vec![],
            },
        );
        assert_eq!(s.game.players[actor].units.len(), 1);
    }
}
