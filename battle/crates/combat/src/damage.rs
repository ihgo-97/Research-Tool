//! Damage formula. Pure functions so they can be unit tested in isolation.

use crate::data::{Balance, MoveDef, TypeChart, TypeId};

/// The attacking side of a hit, captured when the attack is launched so a
/// projectile still resolves correctly if its owner is knocked out.
#[derive(Debug, Clone)]
pub struct AttackerStats {
    pub attack: f32,
    pub types: Vec<TypeId>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DamageResult {
    pub amount: f32,
    pub type_mult: f32,
    pub stab: f32,
}

/// `roll` is uniform in [0, 1) and maps onto the variance window
/// `[1 - variance, 1]`; pass 1.0 for maximum (deterministic) damage.
pub fn compute(
    mv: &MoveDef,
    attacker: &AttackerStats,
    defender_defense: f32,
    defender_types: &[TypeId],
    chart: &TypeChart,
    balance: &Balance,
    roll: f32,
) -> DamageResult {
    let type_mult = chart.multiplier(mv.element, defender_types);
    let stab = if attacker.types.contains(&mv.element) {
        balance.stab
    } else {
        1.0
    };
    let variance = 1.0 - balance.damage_variance * (1.0 - roll.clamp(0.0, 1.0));
    let amount = mv.power * (attacker.attack / defender_defense.max(1.0))
        * balance.damage_scale
        * stab
        * type_mult
        * variance;
    DamageResult {
        amount: amount.max(0.0),
        type_mult,
        stab,
    }
}
