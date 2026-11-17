#[path = "support/control_position.rs"]
mod control_position;
mod support;

#[test]
fn borrowed_units_keep_the_original_deck_multiset_and_attached_gear() {
    let mut equipment_cases = 0;
    for preset in 0..local_rift::cards::presets().len() {
        for owner in 0..2 {
            let (mut game, specs, source, equipment) = control_position::position(preset, owner);
            support::conserved(&game, &specs);
            let borrowed = game
                .take_control_of_battlefield_unit(1 - owner, source)
                .unwrap();
            assert_eq!(game.board_owner(borrowed), Some(owner));
            support::conserved(&game, &specs);
            if let Some(gear) = equipment {
                equipment_cases += 1;
                assert_eq!(game.gear_controller(gear), Some(owner));
            }
            assert_eq!(
                game.take_control_of_battlefield_unit(owner, borrowed),
                Some(source)
            );
            support::conserved(&game, &specs);
        }
    }
    assert!(
        equipment_cases > 0,
        "exercise an opposing controller's attached equipment"
    );
}
