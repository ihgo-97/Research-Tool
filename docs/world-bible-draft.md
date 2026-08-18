# World bible — working draft

**Status: in progress.** This is the scratch/decision-tracking doc, not the
final handoff. Once a section is fully specced in the required formats (see
`design-brief-for-writers.md` section 6), it graduates into `docs/design-spec.md`.

---

## Core loop (confirmed, from original brief)

Wake → check job queue → diagnose/fix on workbench → scavenge for parts →
talk to neighbours / deliver → close up, day-end summary → repeat, harder jobs.

Day is divided into **time blocks** (morning / midday / afternoon / evening),
matching the six-state day loop already in the brief.

## Second pillar: the personal lab

- MC inherited the shop from a father/grandfather/relative who recently
  passed or lost interest and gifted it to him.
- He's already a hobbyist tinkerer before the game starts — has basic
  equipment at his **apartment** (e.g. lighting an LED with a lemon battery —
  low-tier, illustrative-of-character stuff).
- **Early game (roughly the 4th beat, not day one):** he discovers a
  cluttered backroom in the shop full of the relative's old tech and
  half-finished projects. This is the moment the "real" lab pillar unlocks —
  natural power progression from hobbyist-tier to real workshop.
- Lab output feeds the main loop two ways: (1) gadgets that make future
  repair jobs faster/easier, (2) original inventions that open a second
  income/reputation stream — entrepreneurial, inventor framing.
- **STILL OPEN:** exact trigger for the backroom discovery (organic
  exploration? tied to a specific day/job? triggered by dialogue?).

## Comms: phone / computer

- Harvest Moon DS-style: menu-based contact list, tap to call, short
  branching menu (chat / order parts / check job status). NOT rendered in the
  first-person world — a UI-layer content type, separate from world dialogue.
- Used for calling NPCs/vendors, and for **buying parts** (see Parts economy
  below) — the computer is the storefront, browseable without leaving home.
- **STILL OPEN:** full contact list, whether calls can advance events (e.g.
  deliver dialogue) or are transactional-only.

## Relationships

- Harvest Moon-style affinity: every NPC has a friendship/heart-points value,
  raised via gifts, dialogue, favors. Thresholds unlock new dialogue/events.
- Two tracks: **romance** (subset of NPCs, escalating track) and
  **friendship/networking** (everyone else, or in addition to romance).
- Networking has concrete payoff: better prices, rarer parts, referrals,
  future business deals/partnerships — not just flavor.
- Bruno (rival) is the exception: gated by **reputation/skill**, not
  gifts/dialogue — mechanically distinct unlock condition.
- **STILL OPEN:** number of tiers per track, what a typical unlock looks like
  at each tier, total romance-candidate pool size (deliberately left open —
  "as we go on expanding we will add more").

## World structure: city → districts → locations

- **City center + up to ~7 districts** (numbered placeholders for now,
  District 1 = city center/downtown, where the existing map/shop lives).
- Each district has its own locations/maps, built in tiers, not all at once.
- **Commute mechanic:** traveling *between* districts costs one time block;
  moving *within* a district costs nothing/trivial. Sleeping at the apartment
  (District 2) then commuting to the shop (District 1) eats the wake block if
  he dawdles. Strategic but not punishing. Late-game upgrade (bike/transit) —
  buildable in the lab — shrinks commute cost.
- MC's apartment is in **District 2** (adjacent to downtown, working-class
  feel) — NOT the city center.

### Location tiers

**Tier 1 — build now:**
| Location | District | Purpose | Map status |
|---|---|---|---|
| Shop (ext + interior + backroom lab) | 1 | Job queue, workbench, diagnostic minigame, lab discovery beat | **Interior + backroom done** |
| Street block (existing map) | 1 | Ilić, alley dumpster, rival's stall | Exists in engine; needs redraw in District 2's grid style for visual consistency |
| Mr Ilić's place | 1 | First job giver, existing anchor | **Done** |
| Bruno's appliance shop | 1 | Rival/gatekeeper | **Done** (walled-room layout, distinct from MC's shop) |
| Apartment | 2 | Sleep, hobbyist lab, phone/computer hub | **Done** |
| Flower shop | 2 | Flora (owner), Lyra (attendant) | **Done** (open-plan, no counter barrier) |
| Restaurant | **2 (confirmed)** | Dario, Aden, Nora | **Done** (kitchen/dining split via pass-window + staff door) |
| District 2 street (exterior) | 2 | Apartment, flower shop, restaurant placement | **Done** — 4-street grid + alley, see Maps section |

**Tier 2 — next wave:** University, Library, Hotel, Hospital, Junkyard (District 3)
**Tier 3 — backdrop for later:** Plantations, industries, other labs, remaining districts

## Scavenging

Three-tier scavenge economy rather than a single point:

- **Alley dumpster** (District 1, existing) — starter node. Free, close,
  refreshes daily, common/low-tier parts only. Already in v1 scope.
- **Neighbourhood bins / recycling points** — scattered, one or more per
  district. Cheap to add (a new interactive object is hours, not days per
  the brief's cost table). Small yield, common parts. Purpose is to reward
  walking around a district, not just beelining to objectives.
- **Junkyard** (District 3, new location, Tier 2) — a real destination, not
  a quick detour. Costs a commute block like any inter-district travel.
  Rarer/higher-value parts unavailable elsewhere. Possibly its own small
  interior/layout, possibly an NPC who runs or guards it.

**STILL OPEN:**
- Is the junkyard free to access from the start, or gated (permission,
  reputation threshold, or a story beat that reveals District 3)?
- Does scavenging ever fail (search yields nothing), or does every search
  always return something from a pool? — **partially resolved for the
  dumpster**, see Object interactions below; bins/junkyard still open.

## Parts economy: scavenge free/random vs. buy for money

- **Scavenging** (dumpster, bins, junkyard): free, but the specific part
  yielded is randomized from a pool — time cost, no money cost, no certainty.
- **Buying** (via the computer): costs money, but the part is guaranteed —
  no time cost, no uncertainty. The two acquisition methods are deliberately
  opposite trade-offs.
- **Rarity ladder ties acquisition method to part value:**

| Part | Rarity | Source | Buy cost |
|---|---|---|---|
| Wire (generic) | Common | Bins, dumpster, buy | Cheap |
| Screws/fasteners | Common | Bins, dumpster, buy | Cheap |
| Cheap bulb | Common | Dumpster, buy | Cheap |
| Lamp cord | Common | Dumpster, buy | Cheap |
| Switch | Uncommon | Dumpster (low chance), junkyard, buy | Mid |
| Small capacitor | Uncommon | Junkyard, buy | Mid |
| Small motor | Uncommon | Junkyard, buy | Mid |
| Precision sensor | Rare | Junkyard only, or buy at high cost | High |

- Common parts are available almost everywhere, so early game never stalls.
  Uncommon parts nudge the player toward the junkyard or spending money. Rare
  parts make saving up and traveling to the junkyard actually matter later.
- **Recipes** (format matches the brief's event/interaction spec style):

```
id: lamp_repair
inputs: [lamp_cord x1, switch x1, cheap_bulb x1]
built_at: workbench (shop)
output: lamp (repaired) -> delivered to job
```

```
id: basic_nightlight
inputs: [wire x1, cheap_bulb x1, small_capacitor x1]
built_at: lab (backroom or apartment hobbyist bench)
output: nightlight (sellable OR gift item, TBD)
```

- `basic_nightlight` is the placeholder **first lab invention** — small and
  characterful, a step up from the lemon-battery-tier hobbyist stuff.
- **Catalogue scope for v1:** kept deliberately tight — just enough to cover
  the lamp job plus one starter invention. Full device list and tool
  progression stay open per the brief's own scope note.

---

## Object interactions (new, this session)

Every interactive object in a map needs a spec in this format before it
counts as "built," per the brief's own format requirements. This section
defines the schema and runs a few objects through it as a template for the
rest.

### Object categories

Not every object behaves the same way — worth stating as a rule up front so
writers don't assume every object must change game state:

- **Stateful objects** — interacting changes game state (grants an item,
  sets a flag, advances time). E.g. dumpster, workbench, bed.
- **Flavor objects** — interacting only produces a textbox or animation, no
  state change. E.g. decorative shop racks, Ilić's memorabilia shelf. These
  still need a spec entry (so writers know they're *intentionally* inert,
  not unfinished), just a lighter one.

Within stateful objects, a further split:

- **Fixed-result objects** — same interaction always produces the same
  outcome (the bed always advances to the next day).
- **Variable-result objects** — the interaction branches to one of several
  weighted outcomes, including possibly "nothing" (the dumpster).

### Schema

```
id: <object_id>
location: <map_id>
verb: <primary interaction verb, e.g. "search", "use", "sleep", "look">
precondition: <what must be true to interact — time block, item, flag>
cost: <time block / money / none>
outcome_type: <fixed | variable | none>
result:
  - if fixed/none: single outcome
  - if variable: list of {outcome, weight, ui}
ui: <none | textbox | menu | minigame>
cooldown: <none | daily | per-visit>
```

### Worked examples

```
id: alley_dumpster
location: district1_street
verb: search
precondition: none
cost: none
outcome_type: variable
result:
  - outcome: common_part (random from scavenge pool)
    weight: 80% (placeholder — TBD)
    ui: textbox ("You found: [item].")
  - outcome: nothing
    weight: 20% (placeholder — TBD)
    ui: textbox ("Nothing but junk today.")
cooldown: daily
```

```
id: workbench_shop
location: shop_interior
verb: repair | invent
precondition: repair requires an active job in queue; invent requires lab unlocked
cost: none to open; job-specific time cost on completion
outcome_type: fixed (successful minigame always yields the recipe's defined output)
result: opens minigame -> on success, consumes recipe inputs, outputs item/repaired job
ui: minigame, then textbox confirming output
cooldown: none (reusable same day)
```

```
id: apartment_bed
location: apartment_interior
verb: sleep
precondition: none
cost: ends the day (advances to next morning)
outcome_type: fixed
result: day-end summary triggers first, then new day begins
ui: textbox ("You lie down... ") -> transitions to day-end summary screen
cooldown: none
```

```
id: ilic_memorabilia_shelf
location: ilic_place
verb: look
precondition: none
cost: none
outcome_type: none
result: none — flavor only
ui: textbox (descriptive line, no state change)
cooldown: none
```

**STILL OPEN:**
- Dumpster weight split (80/20 above is a placeholder, not a decision).
- Whether neighbourhood bins and the junkyard use the same variable-result
  pattern as the dumpster, and with what weights/pools.
- Full inventory of flavor objects per completed map (shop racks, flower
  displays, restaurant decor, etc. — none spec'd yet).

---

## Text boxes (new, this session)

Three distinct content types were being conflated under "text box." Splitting
them now so writers use the right one:

```
type: world_dialogue
speaker: <npc_id or "narrator">
trigger: <object interaction, NPC approach, event flag>
content: <line(s) of dialogue>
branching: <yes/no — if yes, links to menu>
rendered: in first-person world view
```

```
type: system_text
trigger: <item pickup, job complete, day-end, etc>
content: <short confirmation string>
rendered: overlay, non-diegetic (doesn't break world view)
```

```
type: comms_dialogue
trigger: <phone/computer menu selection>
content: <line(s) of dialogue or menu text>
branching: yes (menu-driven — chat / order parts / check job status)
rendered: UI overlay, NOT first-person world — structurally closer to
  system_text's overlay rendering but with world_dialogue's branching logic
```

**Object interaction textboxes above use `system_text` or `world_dialogue`
depending on context** — e.g. the dumpster's "You found: [item]" is
`system_text` (overlay confirmation), while an NPC reacting to the player
searching in front of them would be `world_dialogue`.

**STILL OPEN:**
- Whether `comms_dialogue` needs its own doc section entirely (it's
  currently folded into the Comms section above) now that it has a distinct
  schema from both other text types.

---

## NPC roster (confirmed)

| NPC | Location | District | Role | Track |
|---|---|---|---|---|
| Mr Ilić | Street / his place | 1 | First job giver, existing anchor | Friendship (mentor) |
| Bruno | Appliance repair shop | 1 | Rival/gatekeeper, niche appliance market, coolly indifferent not hostile — thaws via reputation/skill | Reputation-gated |
| Mrs Flora | Flower shop (owner) | 2 | Shopkeeper | Friendship / networking |
| Lyra | Flower shop (attendant) | 2 | Uni student, popular/beauty type | Romance candidate |
| Dario | Restaurant (owner/cook) | 2 | Warm, old-school, pride in craft | Friendship / networking |
| Aden | Restaurant (waiter) | 2 | Quiet, earnest, room to grow | Friendship (romance? — open) |
| Nora | Restaurant (regular booth) | 2 | Tomboyish, sporty, energetic | Romance candidate |

**Correction logged:** it's **Mr Ilić**, not Mrs — also needs fixing in the
original `design-brief-for-writers.md` worked example, which still says "Mrs
Ilić," so the id/name/gender stay consistent everywhere per the brief's own
"use consistent ids" rule.

**Restaurant district resolved:** District 2, alongside the flower shop and
apartment — gives District 2 three real destinations and an evening/home-life
identity, while District 1 stays work-focused (shop, Ilić, Bruno).

---

## Maps completed this session

Seven interiors plus one full district exterior, each with deliberately
different spatial logic so no two locations feel like a reskin of another:

| Map | Zoning logic | Notable feature |
|---|---|---|
| Shop interior | Public zone blocked by a counter; staff pass through a flap | Two retail racks near the entrance; locked lab door tucked apart from storage |
| Shop backroom / lab | No zoning at all — deliberately cluttered, irregular | Scattered junk piles, covered/tarped workbench, old locked trunk (story hook) |
| Apartment (MC) | Compact single room, zones by function not walls | L-shaped footprint, bathroom as its own enclosed room |
| Mr Ilić's place | Lived-in, settled, no hobby corner (contrast with MC) | Armchair + radio + memorabilia shelf signal decades in one place |
| Bruno's appliance shop | **Fully walled rooms**, not open zones — reception, then a real door into the work floor, then another into storage | Customers never see the work floor at all |
| Flower shop | **Fully open plan**, no barrier at all | Scattered flower displays you browse; checkout counter off to the side, not blocking entry; enclosed cooler room |
| Restaurant | Kitchen/dining split via a **pass-through order window** + separate staff door (a third, distinct barrier type) | Six dining tables to walk between; Nora has a fixed "usual booth" per the schedule-not-pathfinding rule |
| District 2 (exterior street) | Real street grid: Main St + Side St (vertical) crossing Cross St A + Cross St B (horizontal), plus an alley that turns and dead-ends at a dumpster | Apartment, flower shop, restaurant each front a different street; doors labelled with their target interior map id (e.g. `-> apartment_interior`) |

All floor plans use the engine's real constraints: buildings/rooms are solid
exteriors with door markers (not literal interior geometry) per the brief's
"doors are markers, not entrances" rule — exterior shape and interior layout
never need to match, since the interior loads as a completely separate map.

**STILL TO DO:** redraw District 1's existing street (shop, Ilić, Bruno,
alley dumpster) in the same grid-street style as District 2, so both
neighbourhoods share one visual language.

---

## Open questions to resolve next

1. Trigger condition for the backroom/lab discovery event.
2. Contact list contents for the phone/computer.
3. Affinity tier count and unlock pattern (both tracks).
4. Are Dario and/or Aden romance-track, or strictly friendship?
5. Junkyard access — free from the start, or gated behind reputation/story?
6. Dumpster outcome weights (placeholder 80/20 above) — and whether bins and
   junkyard use the same variable-result pattern.
7. Full flavor-object inventory per completed map (not yet spec'd for any
   location).
8. Whether `comms_dialogue` deserves its own doc section.

## Suggested next move

Fully spec **one** slice end-to-end before widening — the brief's own advice
(partial > complete). Strongest candidate: the **lab discovery event**, since
it exercises a map (backroom — already drawn), an event chain
(trigger/condition/effect), and possibly dialogue — the same format types
everything else will need, so it becomes the template for the rest.
