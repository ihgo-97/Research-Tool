# Design brief — what to write, and the shape it has to arrive in

**Read this before writing any game content.** It tells you what the engine can
already do, what is cheap, what is expensive, and the exact format specs need to
be in so they can be implemented without a translation pass.

Hand the finished spec back as `docs/design-spec.md` in this repo.

---

## 1. What the game is

An urban Harvest-Moon-like. You inherit a gutted repair shop on a half-abandoned
city block, learn the trade in public on other people's broken things, and the
block slowly decides whether to trust you.

Modern-day setting, **rendered as a wireframe/holographic first-person view** —
the world looks like a HUD readout of an ordinary city. The aesthetic is the
hook; the world is mundane. Full premise, cast and systems are in
`repairshopgamedesigndoc.md`.

**Core loop:** wake → check the job queue → diagnose and fix on the workbench →
scavenge for missing parts → talk to neighbours / deliver → close up, day-end
summary → repeat with slightly harder jobs.

---

## 2. What exists right now

A single self-contained `index.html` — vanilla JS, one canvas, no build step, no
libraries. Roughly 750 lines. It runs at 60fps on desktop and phone.

### Renderer
First-person raycaster (DDA) drawn as characters in a grid, typically 130×44 on
desktop, 80×77 on a phone. Each screen column casts one ray.

- **Variable wall heights.** Eye is at 0.5; a wall's height is a property of its
  material. Anything under ~0.9 can be seen over.
- **Multiple surfaces per column.** A ray keeps every surface it crosses, so a
  tall building behind a low dumpster still shows above it.
- **Per-material look.** Each material has its own glyph ramp *and* its own
  colour, so different buildings are told apart by texture and hue at a glance.
- Perspective ground grid; empty sky.
- `T` toggles solid/pure-wireframe. `M` toggles the minimap.

### World
`MAP_ART` — the map is ASCII art with a legend, parsed to a numeric grid at load.
Currently 44×22: one city block. Your shop and Mrs Ilić's face each other across
a street, an alley between them dead-ends at a dumpster, a cross street runs
south past the rival's stall, a tall backdrop seals the playable area.

Everything is exterior. Buildings are solid; **doors are markers, not entrances.**

### Controls
Touch-first. Fixed joystick bottom-left (walk + turn), drag anywhere else to
look, tap to interact. Mouse does the same on desktop. Keyboard is a dev
fallback behind a `DEV_KEYS` flag.

### Interaction
Any map cell can be interactive. Looking at one within **2.6 cells** shows a
prompt with its name; tapping it prints its line. Tap resolution is exact —
tapping a dumpster hits the dumpster, tapping just above it hits the wall
behind, tapping low hits the ground.

Every tap fires `window.onWorldTap(ev)`, which is the hook all game logic will
hang off:

```js
{ kind: "prop",                    // wall | prop | ground | sky
  material: "DUMPSTER", height: 0.62,
  world:  { x: 16.5, y: 3 },       // world-space point
  cell:   { x: 16, y: 2, value: 6 },
  distance: 1.4, inReach: true, prop: true,
  from:   { x, y, angle } }        // player pose
```

### Not built yet
No NPCs, no sprites, no dialogue, no inventory, no time or day cycle, no jobs,
no interiors, no save. **Nothing has been written to the screen that isn't a
wall, the ground, or a one-line response.**

---

## 3. Where the visuals are going

Decided: **stay first-person, upgrade the rendering** — textured walls and
billboard sprites instead of character stripes. Think *Lunacid* or
*Dread Delusion*, not Stardew Valley.

This matters for you in one specific way: **NPCs and objects will be billboard
sprites standing at a world position.** They are not tiles on a grid you look
down on. So every character and every placeable object needs a position and a
facing, and needs to make sense viewed from ground level at eye height.

---

## 4. Cheap vs expensive

Write toward the left column. Anything in the right column needs to be worth it.

| Cheap — hours | Expensive — days each |
|---|---|
| A new interactive object (a few characters in the map art) | NPCs who walk routes or follow schedules — needs pathfinding |
| A new building with its own height, colour and texture | Real-time animation or cutscenes — needs a camera system |
| New rooms and map layouts | An inventory/crafting UI — a whole UI layer that doesn't exist |
| Branching dialogue keyed by flags | Physics, projectiles, combat |
| Job queue, repair timers, reputation, day advance | Multiple simultaneous faults in one device |
| The diagnostic minigame as a 2D screen | Weather, seasons, lighting changes |
| Day-end summary screens | Anything needing a server or a save backend |

**Interiors** are mid-cost. The doors exist as markers; making them work means
either a second map you swap to, or interior cells in the same map. One session
either way — worth planning for, don't assume it's free.

**NPCs standing still and talking are cheap. NPCs living lives are not.** For
v1, place each character at a fixed spot and let the day advance move them
between a handful of fixed positions. That reads as a schedule without needing
pathfinding.

---

## 5. Scope for this pass — v1 only

From the design doc's Section 6. Design **only** this:

- The block that exists, plus the **shop interior** and **one other interior**
- **Three NPCs:** Mrs Ilić, one heroine-track character, one rival/gatekeeper
- **One job start to finish** — the lamp — with the day loop wrapped round it
- **One scavenge point** — the alley dumpster
- The **diagnostic minigame** at simple tier: tap test points, each returns
  pass/fail, find the bad component, fit the replacement, retest
- A **day-end summary**

**Leave open** — do not answer these yet: how many romance tracks and how deep,
whether the rival ever becomes an ally, what actually happened to the Tech, the
endgame goal, the full device list, tool progression, economy pacing. These get
decided after someone has played the loop. Designing them now means building
content around a loop nobody has tested.

---

## 6. The format specs must arrive in

Prose has to be interpreted; data gets implemented directly. Use these shapes.

**Use consistent ids everywhere.** If Mrs Ilić is `ilic` in the map, she is
`ilic` in dialogue, in the job table, and in every event.

### Maps
ASCII art plus a legend, one block per map, same convention as the engine uses:

```
map: shop_interior
legend:
  .  walkable floor        #  wall
  W  workbench (interact)  C  counter
  D  door back to street   B  bed / sleep (advances the day)
art:
  ##########
  #........#
  #.W....C.#
  #........#
  #....D####
```
Say the height of anything that isn't a full wall — a counter you see over is
~0.9, a workbench ~0.8.

### NPCs
```
id: ilic
name: Mrs Ilić
sprite: elderly woman, cardigan, holds a lamp   (art direction, one line)
positions:
  morning:   street  @ 22.5, 10.0  facing north
  afternoon: ilic_interior @ 4.5, 6.0  facing south
dialogue:
  first_meeting:  "..."
  repeat:         ["...", "..."]        # cycles
  job_offered:    "..."
  job_in_progress:"..."
  job_delivered:  "..."
```

### Interaction targets
```
id: dumpster
map: street @ 16, 2
prompt: DUMPSTER
default: "Rain and flattened cardboard. Something metal underneath."
search:
  requires: none
  yields: one of [lamp_cord, switch, bulb] — once per day
```

### Events and chains
Trigger → condition → effect. Be explicit about ordering.

```
id: lamp_job_offered
trigger: talk to ilic
condition: day >= 1 AND lamp_job.state == none
effect: set lamp_job.state = accepted; add lamp to queue; ilic.affinity +1
then: lamp_job_diagnose becomes available
```

### The day loop
An explicit list of states and what advances each one:

```
1 wake        -> tap the door / stand up
2 queue       -> shown on the workbench; tap a job to start
3 diagnose    -> minigame; ends in fault identified OR need part
4 scavenge    -> optional branch, back to 3
5 deliver     -> walk to the NPC, talk
6 close       -> tap the bed -> summary screen -> day + 1
```

### Dialogue writing notes
Keep lines short — they render as one or two lines over the world view, not in
a big text box. Say when a line is a one-off versus a repeatable. Mark anything
that changes state.

---

## 7. How to hand it over

Write it as `docs/design-spec.md` and commit it to this repo. Not pasted into a
chat — a file can be diffed when it changes, and read directly at implementation
time.

Partial is fine and preferred. One complete, implementable section beats six
sketched ones. The lamp job end-to-end is the most valuable thing you can
deliver first, because it is the thing that proves whether the loop is fun.
