# Playtest checklist

About 20 minutes. Run `cargo run --release` from `battle/`. Rough notes are
fine. "Too floaty", "Spark feels useless", "Hard AI is unfair" are all useful.

## 1. Controls (first 2-3 fights, any creature vs **easy**)
- [ ] Does movement feel responsive, or slidey/floaty? (`acceleration` in data/balance.ron)
- [ ] Is mouse aiming natural? Do you lose track of where you're facing? (white arrow by your creature)
- [ ] Dash (Space/Shift): does it trigger when you expect? Too short, too long, too frequent? (`dash` in balance.ron)
- [ ] Do you notice the dash i-frames? Can you dash *through* a projectile on purpose? ("dodge" pops up when you do)
- [ ] Holding a move key fires it whenever it's ready. Is that good, or would you rather press once per use?

## 2. Readability (any matchup vs **normal**)
- [ ] Can you tell your creature from the foe at a glance? (white ring = you, red = foe)
- [ ] Can you see enemy attacks coming in time to react? (melee windup arc, burst circles filling up)
- [ ] Are the HUD cooldowns readable mid-fight, or do you never look at them?
- [ ] Do hits feel impactful? (hit-stop, flash, sparks, numbers). Too much or too little? (data/feel.ron)

## 3. Difficulty
- [ ] Easy: should be beatable while learning. Is it?
- [ ] Normal: should be a fair fight. Win rate after ~5 fights?
- [ ] Hard: should need real dodging. Possible? Frustrating?

## 4. Creatures and moves (try each creature at least once)
- [ ] Any creature that feels clearly best or useless?
- [ ] Any move you never use, or only ever use?
- [ ] Do type matchups feel too strong, too weak, or invisible? (orange numbers = super effective)
- [ ] Fight length: the AI averages about 25s. Does that feel right with you playing?

## 5. Anything broken
- [ ] Crashes, stuck creatures, attacks that hit through pillars when they shouldn't, etc.

Quick reference: **Esc/P** pause (R restart, Enter select, X quit) · **R** rematch on the result screen.
