//! VEN-107; expected outcomes from CR355.11 and printed text.
use super::*;
use test_support::{position, ready_unit, reject_unchanged};

const DECREE: CardId = 465;

fn unit(g: &mut Game, owner: usize, card: CardId, might: u64, lane: Option<usize>) -> Target {
    let unit_id = ready_unit(g, owner, card);
    let target = Target::Unit {
        player: owner,
        unit_id,
    };
    let printed = g.unit_might(g.unit_ref(target).unwrap()).unwrap();
    g.modify_might(target, i128::from(might) - i128::from(printed), 0)
        .unwrap();
    g.unit_mut(target).unwrap().lane = lane;
    target
}

fn play(targets: Vec<Target>) -> Action {
    Action::PlayCardWithTargets {
        hand_index: 0,
        lane: None,
        targets,
    }
}

fn settle(g: &mut Game) {
    for _ in 0..80 {
        if g.decision.is_none() && g.chain.is_empty() && g.showdown.is_none() {
            return;
        }
        g.apply(test_support::decline_legend_action(g).unwrap_or(Action::Pass))
            .unwrap();
    }
    panic!("unfinished resolution: {:?}", g.decision);
}

fn response_to_subset(g: &mut Game, target: Target) {
    let caster = g.chain.last().unwrap().player;
    if g.priority() == caster {
        g.apply(Action::Pass).unwrap();
    }
    let opponent = g.priority();
    assert_eq!(opponent, 1 - caster);
    g.players[opponent].hand = vec![7]; // Discipline, actual opposing Reaction.
    g.apply(play(vec![target])).unwrap();
    for _ in 0..60 {
        if matches!(g.decision, Some(Decision::ResolutionSubset { .. })) {
            return;
        }
        g.apply(test_support::decline_legend_action(g).unwrap_or(Action::Pass))
            .unwrap();
    }
    panic!("missing collective-target choice");
}

#[test]
fn discord_initial_targets_use_collective_might_domains_and_distinct_objects() {
    for actor in 0..2 {
        let mut g = position(actor, 0, 46501);
        g.players[actor].hand = vec![DECREE];
        let small = unit(&mut g, 1 - actor, 88, 2, None);
        let large = unit(&mut g, 1 - actor, 89, 3, Some(0));
        let extra = unit(&mut g, 1 - actor, 34, 1, None);
        let friendly = unit(&mut g, actor, 88, 2, None);
        let other_domain = unit(&mut g, 1 - actor, 71, 1, None);
        for targets in [
            vec![small, large, extra],
            vec![small, small],
            vec![friendly],
            vec![other_domain],
        ] {
            reject_unchanged(&mut g, play(targets));
        }
        assert!(
            g.card_target_candidates(actor, DECREE, None, &[small])
                .unwrap()
                .contains(&large)
        );
        assert!(
            !g.card_target_candidates(actor, DECREE, None, &[small, large])
                .unwrap()
                .contains(&extra)
        );
        g.apply(play(vec![small, large])).unwrap();
        settle(&mut g);
        assert!(g.unit_ref(small).is_none() && g.unit_ref(large).is_none());
        assert!(g.unit_ref(extra).is_some() && g.unit_ref(friendly).is_some());
        assert!(g.players[1 - actor].hand.contains(&88) && g.players[1 - actor].hand.contains(&89));
    }
}

#[test]
fn discord_actual_response_requires_controller_choice_and_retains_zero_might_targets() {
    for actor in 0..2 {
        let mut g = position(actor, 0, 46502);
        g.players[actor].hand = vec![DECREE];
        let zero = unit(&mut g, 1 - actor, 88, 0, None);
        let two = unit(&mut g, 1 - actor, 88, 2, None);
        let three = unit(&mut g, 1 - actor, 89, 3, None);
        let unchosen = unit(&mut g, 1 - actor, 34, 1, None);
        g.apply(play(vec![zero, two, three])).unwrap();
        response_to_subset(&mut g, two);
        assert_eq!(g.priority(), actor);
        assert_eq!(g.unit_might(g.unit_ref(two).unwrap()).unwrap(), 4);
        for targets in [
            vec![],
            vec![two],
            vec![three],
            vec![two, three, zero],
            vec![two, zero, unchosen],
            vec![zero, zero, two],
        ] {
            reject_unchanged(&mut g, Action::ChooseResolutionSubset { targets });
        }
        reject_unchanged(&mut g, Action::Choose { accept: false });
        reject_unchanged(&mut g, Action::EndTurn);
        g.apply(Action::ChooseResolutionSubset {
            targets: vec![two, zero],
        })
        .unwrap();
        settle(&mut g);
        assert!(g.unit_ref(two).is_none() && g.unit_ref(zero).is_none());
        assert!(g.unit_ref(three).is_some() && g.unit_ref(unchosen).is_some());
        assert!(g.players[actor].discard.contains(&DECREE));
    }
}

#[test]
fn discord_maximal_subset_does_not_require_globally_largest_cardinality() {
    let mut g = position(0, 0, 46503);
    g.players[0].hand = vec![DECREE];
    let big = unit(&mut g, 1, 88, 2, None);
    let small: Vec<_> = (0..3).map(|_| unit(&mut g, 1, 34, 1, None)).collect();
    g.apply(play([vec![big], small.clone()].concat())).unwrap();
    response_to_subset(&mut g, big);
    let mut fewer = g.clone();
    fewer
        .apply(Action::ChooseResolutionSubset {
            targets: vec![big, small[0]],
        })
        .unwrap();
    settle(&mut fewer);
    assert!(fewer.unit_ref(big).is_none() && fewer.unit_ref(small[0]).is_none());
    assert!(fewer.unit_ref(small[1]).is_some());
    g.apply(Action::ChooseResolutionSubset {
        targets: small.clone(),
    })
    .unwrap();
    settle(&mut g);
    assert!(g.unit_ref(big).is_some());
    assert!(small.iter().all(|&t| g.unit_ref(t).is_none()));
}

#[test]
fn discord_zero_targets_and_more_than_255_zero_might_targets_are_legal() {
    let mut g = position(0, 0, 46504);
    g.players[0].hand = vec![DECREE, DECREE];
    g.apply(play(vec![])).unwrap();
    settle(&mut g);
    let all: Vec<_> = (0..260).map(|_| unit(&mut g, 1, 88, 0, None)).collect();
    g.apply(play(all.clone())).unwrap();
    settle(&mut g);
    assert!(all.iter().all(|&t| g.unit_ref(t).is_none()));
    assert_eq!(g.players[1].hand.len(), 260);
}

#[test]
fn discord_removed_protected_and_huge_targets_are_rechecked_without_overflow() {
    let mut g = position(0, 0, 46505);
    g.players[0].hand = vec![DECREE];
    let gone = unit(&mut g, 1, 88, 1, None);
    let protected = unit(&mut g, 1, 88, 1, None);
    let huge = unit(&mut g, 1, 88, 1, None);
    let valid = unit(&mut g, 1, 88, 1, None);
    g.apply(play(vec![gone, protected, huge, valid])).unwrap();
    // Independent resolution-state mutations complement the real Reaction test.
    g.return_unit_to_hand(gone);
    g.unit_mut(protected).unwrap().protected = true;
    g.unit_mut(huge).unwrap().buff = 1u64 << 40;
    settle(&mut g);
    assert!(g.unit_ref(valid).is_none());
    assert!(g.unit_ref(huge).is_some() && g.unit_ref(protected).is_some());
}

#[test]
fn discord_subset_suggestions_are_legal_and_do_not_mutate_the_position() {
    for actor in 0..2 {
        let mut g = position(actor, 0, 46506);
        g.players[actor].hand = vec![DECREE];
        let a = unit(&mut g, 1 - actor, 88, 2, None);
        let b = unit(&mut g, 1 - actor, 89, 3, None);
        g.apply(play(vec![a, b])).unwrap();
        response_to_subset(&mut g, a);
        let before = format!("{g:?}");
        let actions = g.legal_actions().unwrap();
        assert!(!actions.is_empty());
        assert_eq!(format!("{g:?}"), before);
        for action in actions {
            let Action::ChooseResolutionSubset { ref targets } = action else {
                panic!("wrong decision action")
            };
            g.validate_resolution_subset(targets).unwrap();
            let mut copy = g.clone();
            copy.apply(action).unwrap();
            settle(&mut copy);
            assert_eq!(copy.players[1 - actor].units.len(), 1);
        }
    }
}

#[test]
fn discord_all_ai_levels_finish_subset_choices_without_hidden_information() {
    for actor in 0..2 {
        for difficulty in [
            Difficulty::Easy,
            Difficulty::Tactical,
            Difficulty::Strategic,
        ] {
            let mut g = position(actor, 0, 46507);
            g.difficulty = difficulty;
            g.players[actor].hand = vec![DECREE];
            let zero = unit(&mut g, 1 - actor, 88, 0, None);
            let two = unit(&mut g, 1 - actor, 88, 2, None);
            let three = unit(&mut g, 1 - actor, 89, 3, None);
            g.apply(play(vec![zero, two, three])).unwrap();
            response_to_subset(&mut g, two);

            let before = format!("{g:?}");
            let chosen = g.choose_ai_action().unwrap();
            assert_eq!(format!("{g:?}"), before);
            let Action::ChooseResolutionSubset { ref targets } = chosen else {
                panic!("{difficulty:?} failed to choose remaining targets: {chosen:?}");
            };
            // Either nonzero target can legally accompany zero; both cannot fit.
            assert!(targets.contains(&zero));
            assert_eq!(targets.len(), 2);
            assert!(targets.contains(&two) ^ targets.contains(&three));
            let mut hidden = g.clone();
            hidden.players[1 - actor].hand = vec![1, 4, 5];
            hidden.players[1 - actor].deck.reverse();
            assert_eq!(chosen, hidden.choose_ai_action().unwrap());

            g.apply(chosen).unwrap();
            settle(&mut g);
            assert!(g.unit_ref(zero).is_none());
            assert_eq!(g.players[1 - actor].units.len(), 1);
            assert!(g.players[actor].discard.contains(&DECREE));
        }
    }
}
