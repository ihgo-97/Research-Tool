//! Unit tests for damage, type multipliers, cooldowns, hitboxes and the sim.

use combat::damage::{self, AttackerStats};
use combat::data::GameData;
use combat::hitbox::{arc_hits_circle, circles_overlap, segment_hits_circle, shape_hits_circle, Shape};
use combat::math::Vec2;
use combat::sim::{Action, Battle, Event, Intent, MoveSlot};
use std::f32::consts::FRAC_PI_4;
use std::sync::Arc;

fn data() -> Arc<GameData> {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../data");
    Arc::new(GameData::load_dir(dir).expect("shipped data must load"))
}

/// Walk fighter 0 toward fighter 1 until they are `dist` apart.
fn close_in(b: &mut Battle, dist: f32) {
    for _ in 0..600 {
        let (p0, p1) = (b.fighter(0).body.pos, b.fighter(1).body.pos);
        if p0.distance(p1) <= dist {
            return;
        }
        let walk = Intent { move_dir: (p1 - p0).normalize_or_zero(), aim: p1, ..Default::default() };
        b.step(&[walk, Intent::default()]);
    }
    panic!("never closed in");
}

fn approx(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-3
}

// ------------------------------------------------------------------- data

#[test]
fn shipped_data_loads_and_cross_references_resolve() {
    let d = data();
    assert!(d.creatures.len() >= 4 && d.creatures.len() <= 6);
    assert!(d.moves.len() >= 12);
    for c in &d.creatures {
        assert_eq!(c.moves.len(), 4, "{} should have 4 moves", c.id);
    }
    assert!(d.ai_profile("normal").is_some());
}

#[test]
fn unknown_move_reference_is_rejected() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../data/");
    let read = |f: &str| std::fs::read_to_string(format!("{dir}{f}")).unwrap();
    let creatures = read("creatures.ron").replace("\"ember\"", "\"no_such_move\"");
    let r = GameData::from_strs(
        &read("types.ron"),
        &read("moves.ron"),
        &creatures,
        &read("balance.ron"),
        &read("arena.ron"),
    );
    assert!(r.is_err());
}

// ------------------------------------------------------------ type chart

#[test]
fn type_multipliers_match_chart() {
    let d = data();
    let t = |n: &str| d.types.id(n).unwrap();
    let m = &d.types.multipliers;
    assert!(approx(d.types.multiplier(t("water"), &[t("fire")]), m.strong));
    assert!(approx(d.types.multiplier(t("fire"), &[t("water")]), m.weak));
    assert!(approx(d.types.multiplier(t("electric"), &[t("rock")]), m.immune));
    assert!(approx(d.types.multiplier(t("normal"), &[t("fire")]), 1.0));
    // Dual types multiply.
    assert!(approx(d.types.multiplier(t("water"), &[t("fire"), t("rock")]), m.strong * m.strong));
}

#[test]
fn type_chart_is_never_a_hard_counter() {
    let d = data();
    let n = d.types.names.len();
    for a in 0..n {
        for b in 0..n {
            assert!(d.types.single(a, b) >= 0.5, "{} vs {}", d.types.names[a], d.types.names[b]);
        }
    }
}

// ----------------------------------------------------------------- damage

#[test]
fn damage_formula_applies_stats_stab_type_and_variance() {
    let d = data();
    let ember = &d.moves[d.move_id("ember").unwrap()];
    let fire = d.types.id("fire").unwrap();
    let grass = d.types.id("grass").unwrap();
    let normal = d.types.id("normal").unwrap();
    let b = &d.balance;

    let neutral = AttackerStats { attack: 100.0, types: vec![normal] };
    let r = damage::compute(ember, &neutral, 100.0, &[normal], &d.types, b, 1.0);
    assert!(approx(r.amount, ember.power * b.damage_scale));
    assert!(approx(r.stab, 1.0));

    let stab = AttackerStats { attack: 100.0, types: vec![fire] };
    let r = damage::compute(ember, &stab, 100.0, &[grass], &d.types, b, 1.0);
    let strong = d.types.multipliers.strong;
    assert!(approx(r.amount, ember.power * b.damage_scale * b.stab * strong));
    assert!(approx(r.type_mult, strong));

    // Attack/defense ratio scales linearly.
    let r2 = damage::compute(ember, &neutral, 200.0, &[normal], &d.types, b, 1.0);
    assert!(approx(r2.amount, ember.power * b.damage_scale * 0.5));

    // Lowest roll removes exactly `damage_variance`.
    let lo = damage::compute(ember, &neutral, 100.0, &[normal], &d.types, b, 0.0);
    assert!(approx(lo.amount, ember.power * b.damage_scale * (1.0 - b.damage_variance)));
}

// -------------------------------------------------------------- cooldowns

#[test]
fn move_slot_cooldown_ticks_to_ready() {
    let mut s = MoveSlot { mv: 0, remaining: 0.5 };
    assert!(!s.ready());
    s.tick(0.3);
    assert!(!s.ready());
    s.tick(0.3);
    assert!(s.ready());
    assert_eq!(s.remaining, 0.0);
}

#[test]
fn using_a_move_starts_its_cooldown_and_blocks_reuse() {
    let d = data();
    let a = d.creature_id("emberkit").unwrap();
    let mut b = Battle::new(d.clone(), &[a, a], 1);
    let aim = b.fighter(1).body.pos;
    let use0 = Intent { aim, use_move: Some(0), ..Default::default() };
    b.step(&[use0, Intent::default()]);
    let f = b.fighter(0).fighter;
    let mv = &d.moves[f.slots[0].mv];
    assert!(approx(f.slots[0].remaining, mv.cooldown));
    assert!(b.events().iter().any(|e| matches!(e, Event::MoveStarted { fighter: 0, .. })));

    // Run until idle again, keep requesting: must not restart before cooldown expires.
    let mut restarted_at = None;
    for _ in 0..(d.balance.tick_rate * 10) {
        b.step(&[use0, Intent::default()]);
        if b.events().iter().any(|e| matches!(e, Event::MoveStarted { fighter: 0, .. })) {
            restarted_at = Some(b.time());
            break;
        }
    }
    let t = restarted_at.expect("move should become usable again");
    assert!(t >= mv.cooldown - 1e-3, "restarted at {t}, cooldown {}", mv.cooldown);
}

#[test]
fn dash_respects_cooldown_and_grants_iframes() {
    let d = data();
    let a = d.creature_id("tidepup").unwrap();
    let mut b = Battle::new(d.clone(), &[a, a], 1);
    let dash = Intent { move_dir: Vec2::new(0.0, 1.0), dash: true, ..Default::default() };
    b.step(&[dash, Intent::default()]);
    let f = b.fighter(0).fighter;
    assert!(matches!(f.action, Action::Dash { .. }));
    assert!(f.iframes > 0.0);
    assert!(approx(f.dash_cooldown, d.balance.dash.cooldown));
    // Second dash request right after the first ends must be refused.
    for _ in 0..((d.balance.dash.duration * d.balance.tick_rate as f32) as usize + 2) {
        b.step(&[dash, Intent::default()]);
    }
    assert_eq!(b.fighter(0).fighter.stats.dashes, 1);
}

// ---------------------------------------------------------------- hitboxes

#[test]
fn circle_collision() {
    assert!(circles_overlap(Vec2::ZERO, 10.0, Vec2::new(19.0, 0.0), 10.0));
    assert!(!circles_overlap(Vec2::ZERO, 10.0, Vec2::new(21.0, 0.0), 10.0));
}

#[test]
fn arc_collision_respects_range_and_angle() {
    let o = Vec2::ZERO;
    let dir = Vec2::X;
    let half = FRAC_PI_4; // 90 degree arc
    assert!(arc_hits_circle(o, 60.0, dir, half, Vec2::new(50.0, 0.0), 10.0));
    assert!(!arc_hits_circle(o, 60.0, dir, half, Vec2::new(-50.0, 0.0), 10.0), "behind");
    assert!(!arc_hits_circle(o, 60.0, dir, half, Vec2::new(0.0, 50.0), 5.0), "to the side");
    assert!(!arc_hits_circle(o, 60.0, dir, half, Vec2::new(80.0, 0.0), 10.0), "too far");
    assert!(arc_hits_circle(o, 60.0, dir, half, Vec2::new(65.0, 0.0), 10.0), "edge touches");
    // A large target just outside the angle still gets clipped by its radius.
    let p = Vec2::from_angle(half + 0.2) * 40.0;
    assert!(arc_hits_circle(o, 60.0, dir, half, p, 15.0));
    assert!(!arc_hits_circle(o, 60.0, dir, half, p, 2.0));
    // Arc wraps correctly around +/-PI.
    assert!(arc_hits_circle(o, 60.0, -Vec2::X, half, Vec2::new(-50.0, 1.0), 5.0));
    assert!(shape_hits_circle(&Shape::Arc { range: 60.0, dir, half_arc: half }, o, Vec2::new(30.0, 0.0), 5.0));
}

#[test]
fn segment_line_of_sight() {
    let c = Vec2::new(50.0, 0.0);
    assert!(segment_hits_circle(Vec2::ZERO, Vec2::new(100.0, 0.0), c, 10.0));
    assert!(!segment_hits_circle(Vec2::ZERO, Vec2::new(100.0, 30.0), c, 10.0));
    assert!(!segment_hits_circle(Vec2::ZERO, Vec2::new(30.0, 0.0), c, 10.0), "segment stops short");
}

// --------------------------------------------------------------- simulation

#[test]
fn projectile_hits_and_damages_target() {
    let d = data();
    let a = d.creature_id("voltfin").unwrap();
    let mut b = Battle::new(d.clone(), &[a, a], 3);
    close_in(&mut b, 350.0);
    let target = b.fighter(1).body.pos;
    let hp0 = b.fighter(1).fighter.hp;
    let shoot = Intent { aim: target, use_move: Some(0), ..Default::default() };
    let mut hit = false;
    for _ in 0..120 {
        b.step(&[shoot, Intent::default()]);
        if b.events().iter().any(|e| matches!(e, Event::Hit { attacker: 0, target: 1, .. })) {
            hit = true;
            break;
        }
    }
    assert!(hit, "spark should cross the arena and hit");
    assert!(b.fighter(1).fighter.hp < hp0);
}

#[test]
fn dashing_through_a_projectile_avoids_damage() {
    let d = data();
    let a = d.creature_id("voltfin").unwrap();
    let mut b = Battle::new(d.clone(), &[a, a], 3);
    close_in(&mut b, 350.0);
    let target = b.fighter(1).body.pos;
    let shoot = Intent { aim: target, use_move: Some(0), ..Default::default() };
    // Find the tick the projectile would hit by simulating a copy first.
    let mut probe = Battle::new(d.clone(), &[a, a], 3);
    close_in(&mut probe, 350.0);
    let mut hit_tick = 0;
    for t in 0..120 {
        probe.step(&[shoot, Intent::default()]);
        if probe.events().iter().any(|e| matches!(e, Event::Hit { .. })) {
            hit_tick = t;
            break;
        }
    }
    assert!(hit_tick > 2);
    let hp0 = b.fighter(1).fighter.hp;
    let mut dodged = false;
    // Dash *into* the projectile a couple of ticks before impact; iframes carry us through.
    let toward = (b.fighter(0).body.pos - target).normalize_or_zero();
    for t in 0..hit_tick + 20 {
        let now = t == hit_tick - 2;
        let dash = Intent { dash: now, move_dir: if now { toward } else { Vec2::ZERO }, ..Default::default() };
        b.step(&[shoot, dash]);
        dodged |= b.events().iter().any(|e| matches!(e, Event::Dodged { fighter: 1, .. }));
    }
    assert!(dodged);
    assert_eq!(b.fighter(1).fighter.hp, hp0);
}

#[test]
fn ai_battles_finish_and_are_deterministic() {
    let d = data();
    let n = d.creatures.len();
    for a in 0..n {
        let b = (a + 1) % n;
        let s1 = combat::run_ai_battle(&d, a, b, "normal", 42);
        let s2 = combat::run_ai_battle(&d, a, b, "normal", 42);
        assert!(s1.seconds <= d.balance.max_battle_seconds + 0.1);
        assert_eq!(s1.outcome, s2.outcome);
        assert_eq!(s1.seconds, s2.seconds);
        assert!(s1.stats[0].hits_landed + s1.stats[1].hits_landed > 0, "nobody landed a hit");
    }
}

// ----------------------------------------------------------------- statuses

/// Shipped data, but every status proc always triggers.
fn data_sure_procs() -> Arc<GameData> {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../data/");
    let read = |f: &str| std::fs::read_to_string(format!("{dir}{f}")).unwrap();
    let moves = regex_lite_chance(&read("moves.ron"));
    Arc::new(
        GameData::from_strs(&read("types.ron"), &moves, &read("creatures.ron"), &read("balance.ron"), &read("arena.ron")).unwrap(),
    )
}

fn regex_lite_chance(s: &str) -> String {
    // Replace every `chance: <number>` with `chance: 1.0`.
    let mut out = String::new();
    let mut rest = s;
    while let Some(i) = rest.find("chance: ") {
        out.push_str(&rest[..i + 8]);
        rest = &rest[i + 8..];
        let end = rest.find(|c: char| !(c.is_ascii_digit() || c == '.')).unwrap_or(rest.len());
        out.push_str("1.0");
        rest = &rest[end..];
    }
    out.push_str(rest);
    out
}

/// Fire slot `slot` of fighter 0 at fighter 1 until it lands; returns events seen.
fn land_hit(b: &mut Battle, slot: usize) -> Vec<Event> {
    let mut seen = Vec::new();
    for _ in 0..240 {
        let aim = b.fighter(1).body.pos;
        b.step(&[Intent { aim, use_move: Some(slot), ..Default::default() }, Intent::default()]);
        seen.extend_from_slice(b.events());
        if b.events().iter().any(|e| matches!(e, Event::Hit { attacker: 0, .. })) {
            return seen;
        }
    }
    panic!("never landed a hit");
}

#[test]
fn burn_deals_damage_over_time_then_expires() {
    let d = data_sure_procs();
    let fire = d.creature_id("emberkit").unwrap();
    let target = d.creature_id("tidepup").unwrap();
    let mut b = Battle::new(d.clone(), &[fire, target], 1);
    close_in(&mut b, 300.0);
    let seen = land_hit(&mut b, 0); // Ember
    assert!(seen.iter().any(|e| matches!(e, Event::StatusApplied { fighter: 1, .. })));
    let burn = d.status_id("burn").unwrap();
    assert_eq!(b.fighter(1).fighter.status.map(|s| s.status), Some(burn));
    let hp = b.fighter(1).fighter.hp;
    let mut ended = false;
    for _ in 0..(d.statuses[burn].duration * d.balance.tick_rate as f32) as usize + 5 {
        b.step(&[Intent::default(), Intent::default()]);
        ended |= b.events().iter().any(|e| matches!(e, Event::StatusEnded { fighter: 1, broken: false, .. }));
    }
    let lost = hp - b.fighter(1).fighter.hp;
    let expected = d.statuses[burn].damage_per_sec * d.statuses[burn].duration * b.fighter(1).fighter.max_hp;
    assert!(ended, "burn should expire");
    assert!((lost - expected).abs() < expected * 0.1, "lost {lost}, expected about {expected}");
    assert!(b.fighter(1).fighter.status.is_none());
}

#[test]
fn sleep_blocks_actions_and_breaks_on_hit() {
    let d = data_sure_procs();
    let grass = d.creature_id("sproutle").unwrap();
    let target = d.creature_id("pebblor").unwrap();
    let mut b = Battle::new(d.clone(), &[grass, target], 1);
    close_in(&mut b, 60.0);
    land_hit(&mut b, 2); // Spore Burst
    let sleep = d.status_id("sleep").unwrap();
    assert_eq!(b.fighter(1).fighter.status.map(|s| s.status), Some(sleep));
    // Let knockback settle, then check a sleeping fighter ignores move and dash input.
    for _ in 0..40 {
        b.step(&[Intent::default(), Intent::default()]);
    }
    let p0 = b.fighter(1).body.pos;
    let aim = b.fighter(0).body.pos;
    for _ in 0..20 {
        let wake_attempt = Intent { aim, use_move: Some(0), dash: true, move_dir: Vec2::X, ..Default::default() };
        b.step(&[Intent::default(), wake_attempt]);
        assert!(!b.events().iter().any(|e| matches!(e, Event::MoveStarted { fighter: 1, .. } | Event::Dash { fighter: 1 })));
    }
    assert!(b.fighter(1).body.pos.distance(p0) < 3.0, "sleeper should not walk");
    // Wait for Tackle (slot 3) and hit: the sleeper wakes up.
    let seen = land_hit(&mut b, 3);
    assert!(seen.iter().any(|e| matches!(e, Event::StatusEnded { fighter: 1, broken: true, .. })));
}

#[test]
fn type_immunity_prevents_status() {
    let d = data_sure_procs();
    let fire = d.creature_id("emberkit").unwrap();
    let mut b = Battle::new(d.clone(), &[fire, fire], 1);
    close_in(&mut b, 300.0);
    let seen = land_hit(&mut b, 0);
    assert!(!seen.iter().any(|e| matches!(e, Event::StatusApplied { .. })), "fire types can't be burned");
}

// ------------------------------------------------------------- team battles

fn trio(d: &GameData, ids: [&str; 3]) -> Vec<usize> {
    ids.iter().map(|i| d.creature_id(i).unwrap()).collect()
}

#[test]
fn voluntary_switch_swaps_in_place_with_cooldown() {
    let d = data();
    let rosters = [trio(&d, ["emberkit", "tidepup", "sproutle"]), trio(&d, ["voltfin", "pebblor", "galeon"])];
    let mut b = Battle::with_rosters(d.clone(), &rosters, 1);
    assert_eq!(b.active(0), 0);
    assert!(b.fighter(1).fighter.benched && b.fighter(2).fighter.benched);
    // Walk a bit so the swap position is distinguishable from the spawn.
    for _ in 0..30 {
        b.step(&[Intent { move_dir: Vec2::new(1.0, 0.0), ..Default::default() }, Intent::default()]);
    }
    let pos = b.fighter(0).body.pos;
    b.step(&[Intent { switch_to: Some(2), ..Default::default() }, Intent::default()]);
    assert!(b.events().iter().any(|e| matches!(e, Event::SentOut { team: 0, from: 0, to: 2, voluntary: true })));
    assert_eq!(b.active(0), 2);
    let new = b.fighter(2);
    assert!(!new.fighter.benched && new.fighter.iframes > 0.0);
    assert!(new.body.pos.distance(pos) < 10.0, "comes in where the old one stood");
    assert!(b.fighter(0).fighter.benched);
    // Cooldown blocks an immediate switch back.
    b.step(&[Intent { switch_to: Some(0), ..Default::default() }, Intent::default()]);
    assert_eq!(b.active(0), 2);
}

#[test]
fn benched_fighters_cannot_be_hit_and_keep_recovering() {
    let d = data();
    let rosters = [trio(&d, ["voltfin", "tidepup", "sproutle"]), trio(&d, ["emberkit", "pebblor", "galeon"])];
    let mut b = Battle::with_rosters(d.clone(), &rosters, 2);
    // Use a move, then switch out: the benched creature's cooldown still ticks.
    let aim = b.fighter(b.active(1)).body.pos;
    b.step(&[Intent { aim, use_move: Some(1), ..Default::default() }, Intent::default()]);
    let cd = b.fighter(0).fighter.slots[1].remaining;
    for _ in 0..60 {
        b.step(&[Intent { switch_to: Some(1), ..Default::default() }, Intent::default()]);
    }
    assert!(b.fighter(0).fighter.benched);
    assert!(b.fighter(0).fighter.slots[1].remaining < cd - 0.9);
}

#[test]
fn knocked_out_creature_is_replaced_and_last_knockout_ends_battle() {
    let d = data();
    let rosters = [trio(&d, ["emberkit", "tidepup", "sproutle"]), vec![d.creature_id("galeon").unwrap()]];
    let profile = d.ai_profile("hard").unwrap().clone();
    let mut b = Battle::with_rosters(d.clone(), &rosters, 3);
    let mut ai0 = combat::ai::AiController::new(profile.clone(), 1);
    let mut ai1 = combat::ai::AiController::new(profile, 2);
    let mut sent_after_faint = 0;
    while b.outcome().is_none() && b.time() < 200.0 {
        let i = [ai0.update(&b, 0), ai1.update(&b, 1)];
        b.step(&i);
        sent_after_faint += b.events().iter().filter(|e| matches!(e, Event::SentOut { voluntary: false, .. })).count();
    }
    let out = b.outcome().expect("battle ends");
    if out.winner() == Some(1) {
        assert_eq!(sent_after_faint, 2, "both replacements were sent out before team 0 lost");
        assert!(b.teams()[0].members.iter().all(|&m| !b.fighter(m).fighter.alive()));
    } else {
        assert!(!b.fighter(3).fighter.alive());
    }
}

#[test]
fn ai_team_battles_finish_and_are_deterministic() {
    let d = data();
    let rosters = [trio(&d, ["emberkit", "sproutle", "pebblor"]), trio(&d, ["tidepup", "voltfin", "galeon"])];
    for seed in 0..4 {
        let a = combat::run_ai_team_battle(&d, rosters.clone(), ["normal", "normal"], seed);
        let b = combat::run_ai_team_battle(&d, rosters.clone(), ["normal", "normal"], seed);
        assert_eq!(a.outcome, b.outcome);
        assert_eq!(a.seconds, b.seconds);
        assert_eq!(a.switches, b.switches);
        assert!(a.seconds <= d.balance.max_battle_seconds * 3.0 + 0.1);
    }
}
