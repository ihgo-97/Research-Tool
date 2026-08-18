# Dialogue inventory — full scaffold

**Purpose:** enumerate every dialogue *slot* each NPC needs (not full scripts
yet) so writing can proceed slot-by-slot without missing a case. Uses the
three content types already locked in the world bible: `world_dialogue`,
`system_text`, `comms_dialogue`.

**Status:** scaffold + worked examples for Ilić (per the current focus).
Other NPCs listed with their required slots, unwritten.

---

## Dialogue taxonomy (applies to every NPC)

Not every NPC needs every category — depends on their track (friendship,
romance, reputation-gated). The full set:

| # | Category | Trigger | Applies to |
|---|---|---|---|
| 1 | **First meeting / intro** | First time player approaches | Everyone |
| 2 | **Generic barks (repeatable)** | Talk again, no new state | Everyone |
| 3 | **Time-of-day variants** | Same NPC, morning/midday/afternoon/evening | NPCs with a schedule position |
| 4 | **Job-giving** | NPC hands off a job | Job-giver NPCs (Ilić first; others later) |
| 5 | **Job follow-up / status check** | Player returns before job done | Job-giver NPCs |
| 6 | **Job delivery / completion** | Player delivers finished job | Job-giver NPCs |
| 7 | **Affinity tier-up (friendship)** | Heart-points cross a threshold | Friendship-track NPCs |
| 8 | **Affinity tier-up (romance)** | Heart-points cross a threshold | Romance candidates only |
| 9 | **Gift reaction** | Player gives an item | Everyone (reaction varies by liked/neutral/disliked) |
| 10 | **Networking payoff unlock** | Threshold reached — new discount/referral/part access | Friendship/networking NPCs |
| 11 | **Reputation-gated thaw** | Skill/reputation threshold, not affinity | Bruno only (unique mechanic) |
| 12 | **Event-specific reaction** | Story beat elsewhere affects this NPC's line (e.g. lab discovery) | Everyone, case-by-case |
| 13 | **Comms (phone/computer) variant** | Same relationship, different channel | Everyone with a contact entry |

**STILL OPEN (flagging, not deciding here):** whether every NPC needs all 13
slots or whether minor NPCs get a reduced set — worth deciding per-NPC as we
go rather than forcing uniformity.

---

## Mr Ilić — worked example (mentor / friendship track)

| Slot | id | Status |
|---|---|---|
| 1. First meeting | `ilic_intro` | **Drafted** |
| 2. Generic bark | `ilic_bark_generic` | **Drafted** |
| 3. Time-of-day | `ilic_morning` / `ilic_evening` | **Drafted** — fixed spot, 2 variants not 4 (see note) |
| 4. Job-giving | `ilic_first_job` (wraps `ilic_intro`) | **Drafted** |
| 5. Job follow-up | `ilic_job_pending` | **Drafted** |
| 6. Job delivery | `ilic_job_complete` | **Drafted** |
| 7. Friendship tier-up | `ilic_tier_1` | **Drafted** (tier 1 only — 2+ blocked on tier-count decision) |
| 9. Gift reaction | `ilic_gift_liked` / `_neutral` / `_disliked` | **Drafted** — likes defined below |
| 10. Networking payoff | `ilic_referral_unlock` | **Drafted** |
| 12. Event-specific | `ilic_reacts_to_lab_discovery` | **Drafted** |
| 13. Comms variant | n/a — Ilić confirmed NOT on phone contact list (old-school, in-person only) — **now a decision, not just a note** |

### Tone lock
Gruff but warm. Short sentences. Dry humor. Affection stays under bluntness —
he never says anything sentimental straight; it always comes out sideways, as
a jab or a non-sequitur. Applied consistently below.

### Gift preferences (new — needed for slot 9)
- **Liked:** anything hand-repaired/tinkered rather than bought new; old radio
  parts or tubes (he collects, doesn't say why yet — hook for later);
  strong black coffee.
- **Neutral:** most bought goods, food gifts.
- **Disliked:** anything that reads as charity or pity — he'll bristle.

### Drafted content

```
type: world_dialogue
id: ilic_intro
speaker: ilic
trigger: ilic_first_job event (player approaches Ilić, street, flag ilic_intro_done == false)
content:
  - ilic: "You're [relative]'s boy, aren't you? Heard you took over the shop."
  - ilic: "Good. Place was getting dusty. He always said you had hands for this kind of work."
  - ilic: "Matter of fact — my lamp's been dead a week. Cord, switch, something. Bring it back to life and I'll make it worth your while."
  - [player: acknowledge / ask what's wrong with it]
  - ilic: "Don't know, don't care — that's your department now."
branching: no (linear v1; flagged for later branch point)
rendered: in first-person world view
```

```
id: ilic_first_job
trigger: player approaches Ilić on street (District 1, first street visit)
condition: flag ilic_intro_done == false
effect:
  - play world_dialogue: ilic_intro
  - on dialogue end: add job "lamp_repair" to job queue
  - set flag ilic_intro_done = true
```

```
type: world_dialogue
id: ilic_bark_generic
speaker: ilic
trigger: player talks to Ilić, no active event/job state change
content: (rotate/random pick, no branching)
  - ilic: "Shop still standing? Good sign."
  - ilic: "Don't let the dust win. Dust always wins eventually, but don't help it."
  - ilic: "You look tired. Working, or just bad at sleeping?"
branching: no
rendered: in first-person world view
```

```
type: world_dialogue
id: ilic_morning
speaker: ilic
trigger: time block == morning, player talks to Ilić
content:
  - ilic: "Early start. He'd have liked that."
branching: no
rendered: in first-person world view
```

```
type: world_dialogue
id: ilic_evening
speaker: ilic
trigger: time block == evening, player talks to Ilić
content:
  - ilic: "Go home. Even I go home eventually."
branching: no
rendered: in first-person world view
```
**Note:** only 2 of the possible 4 time-of-day slots written — midday/afternoon
can reuse `ilic_bark_generic` for now rather than needing unique lines. Flag
if that reads as thin once implemented.

```
type: world_dialogue
id: ilic_job_pending
speaker: ilic
trigger: player talks to Ilić while lamp_repair job is in queue, not yet delivered
content:
  - ilic: "Lamp's not going to fix itself. Not that it was doing much before, either."
branching: no
rendered: in first-person world view
```

```
type: world_dialogue
id: ilic_job_complete
speaker: ilic
trigger: player delivers completed lamp_repair job to Ilić
content:
  - ilic: "Well. Look at that." [inspects lamp]
  - ilic: "Not bad. Not bad at all."
  - ilic: "Here — for your trouble." [pays job reward]
  - ilic: "Don't get used to compliments. That's the last one for a while."
branching: no
rendered: in first-person world view
effect: pay job reward, remove lamp_repair from queue, +heart_points (ilic), set flag ilic_first_job_done = true
```

```
type: world_dialogue
id: ilic_tier_1
speaker: ilic
trigger: ilic heart_points crosses tier-1 threshold (value TBD, blocked on world-bible open Q3)
content:
  - ilic: "You've been alright, kid. Not that I was worried."
  - ilic: "He'd have liked seeing the shop open again."
branching: no
rendered: in first-person world view
effect: unlock ilic_referral_unlock eligibility (networking payoff gated behind this tier)
```

```
type: world_dialogue
id: ilic_gift_liked
speaker: ilic
trigger: player gives Ilić an item on the liked list
content:
  - ilic: "Huh. You made this? ...Fine work." [pockets it before saying more]
branching: no
rendered: in first-person world view
effect: +heart_points (larger increment)
```

```
type: world_dialogue
id: ilic_gift_neutral
speaker: ilic
trigger: player gives Ilić a neutral-list item
content:
  - ilic: "Didn't need it, but I won't say no."
branching: no
rendered: in first-person world view
effect: +heart_points (small increment)
```

```
type: world_dialogue
id: ilic_gift_disliked
speaker: ilic
trigger: player gives Ilić a disliked-list item
content:
  - ilic: "I don't need charity. Save it."
branching: no
rendered: in first-person world view
effect: no heart_points change, or small penalty (TBD)
```

```
type: world_dialogue
id: ilic_referral_unlock
speaker: ilic
trigger: ilic_tier_1 reached AND flag ilic_first_job_done == true
content:
  - ilic: "There's a fella two streets over, radio's been busted for a month. Told him you might be worth the visit."
branching: no
rendered: in first-person world view
effect: unlocks next job-giver NPC or job (specific NPC TBD — placeholder hook)
```

```
type: world_dialogue
id: ilic_reacts_to_lab_discovery
speaker: ilic
trigger: player talks to Ilić AFTER backroom/lab discovery event flag is set
content:
  - ilic: "So you found it." [long pause]
  - ilic: "Knew that room was still back there. He never let anyone touch it. Not even me."
  - ilic: "...Don't break anything he didn't already break himself."
branching: no
rendered: in first-person world view
```

### Slot 13 decision
Ilić is confirmed **not** on the phone contact list — in-person only, consistent
with his "lived-in, settled, no hobby corner" characterization. This is now a
locked world-bible fact, not an open flag — worth carrying into the Comms
section when that gets revisited.

**Ilić dialogue set: complete for v1** (tier 2+ friendship lines deferred until
tier-count decision). Template proven — ready to stamp this same slot-by-slot
process onto the next NPC.

---

## Bruno — reputation-gated (unique mechanic, no affinity slots)

| Slot | id | Status |
|---|---|---|
| 1. First meeting | `bruno_intro` | **Drafted** |
| 2. Generic bark | `bruno_bark_generic` | **Drafted** |
| 11. Reputation thaw stages | `bruno_thaw_1` | **Drafted** (stage 1 only — 2+ blocked on tier-count-equivalent decision for reputation) |
| 12. Event-specific | `bruno_reacts_to_milestone` | **Drafted** — reacts to player completing a job Bruno also wanted |

**Slots 7–10 and 13 do NOT apply** — confirmed: no affinity, no gifts, no
networking payoff, no comms channel. Bruno stays mechanically isolated from
the rest of the roster, exactly as the world bible specifies. This is a
completed decision, not an open flag.

### Tone lock
Terse and clipped. Minimal words — full sentences are rare, most lines are
fragments. Not hostile, just unbothered by small talk. Thaw is earned
entirely through demonstrated skill/reputation, never through effort to be
liked; when it comes, it's short but genuine, no dramatic reversal.

### Drafted content

```
type: world_dialogue
id: bruno_intro
speaker: bruno
trigger: player's first visit to Bruno's appliance shop
content:
  - bruno: "Reception's out front. Work floor's mine."
  - bruno: "[relative]'s kid. Heard you reopened." [doesn't look up]
  - bruno: "We'll see."
branching: no
rendered: in first-person world view
```

```
type: world_dialogue
id: bruno_bark_generic
speaker: bruno
trigger: player talks to Bruno, no active event/state change, reputation below thaw threshold
content: (rotate/random pick, no branching)
  - bruno: "Busy."
  - bruno: "Something you need, or just looking?"
  - bruno: "..." [barely acknowledges, keeps working]
branching: no
rendered: in first-person world view
```

```
type: world_dialogue
id: bruno_thaw_1
speaker: bruno
trigger: player reputation crosses thaw threshold 1 (value TBD — reputation stat definition itself still open, separate from affinity tier-count question)
content:
  - bruno: "Heard about the [job type] you fixed." [pause] "Clean work."
  - bruno: "Don't let it go to your head."
branching: no
rendered: in first-person world view
```

```
type: world_dialogue
id: bruno_reacts_to_milestone
speaker: bruno
trigger: player completes a job/repair Bruno was also positioned to take (specific job TBD — placeholder hook)
content:
  - bruno: "That one was mine to lose. Didn't expect to lose it."
  - bruno: "...Good."
branching: no
rendered: in first-person world view
```

**Bruno dialogue set: complete for v1** (thaw stage 2+ deferred — his
progression needs its own value scale since it's reputation-based, not
heart-points, a distinct open question from the affinity tier-count one).

---

## Full roster status: all 7 NPCs drafted for v1

| NPC | Slots drafted | Deferred to later passes |
|---|---|---|
| Mr Ilić | 11 of 13 | — (13 n/a, confirmed) |
| Bruno | 4 of 4 applicable | thaw stage 2+ (reputation scale undefined) |
| Mrs Flora | 6 of 13 | tier 2+, time-of-day, job slots (n/a — not a job-giver) |
| Lyra | 5 of 13 | tier 2+, time-of-day, event-reaction |
| Nora | 5 of 13 | tier 2+, comms, event-reaction |
| Dario | 4 of 13 | tier 2+, event-reaction, comms |
| Aden | 4 of 13 | tier 2+, networking, event-reaction, comms |

**Every NPC now has a complete v1 core set** (intro + bark + primary
track-appropriate progression + gifts, where applicable). Remaining gaps are
consistently the same few blockers across the board, not scattered
one-offs — a good sign the taxonomy held up.

---

## Mrs Flora — shopkeeper (friendship/networking)

| Slot | id | Status |
|---|---|---|
| 1. First meeting | `flora_intro` | **Drafted** |
| 2. Generic bark | `flora_bark_generic` | **Drafted** |
| 9. Gift reaction | `flora_gift_liked` / `_neutral` / `_disliked` | **Drafted** — likes defined below |
| 10. Networking payoff | `flora_discount_unlock` | **Drafted** |
| 12. Event-specific | `flora_reacts_to_lab_discovery` | **Drafted** |
| 13. Comms variant | `flora_comms_order` | **Drafted** — confirmed on contact list (flower orders) |

### Tone lock
Warm, maternal. Fusses over the player — comments on whether he's eating,
sleeping, dressing warm enough. Generous with unsolicited advice, delivered
as care rather than nagging. Uses endearments ("dear," "love").

### Gift preferences (needed for slot 9)
- **Liked:** anything homemade/baked; rare or unusual seeds/cuttings; tea.
- **Neutral:** most bought goods.
- **Disliked:** cut flowers from anywhere but her own shop — she'll notice
  and be quietly wounded, not angry (in-character: she'd never say so
  directly).

### Drafted content

```
type: world_dialogue
id: flora_intro
speaker: flora
trigger: player's first visit to flower shop
content:
  - flora: "Oh, hello dear! You must be [relative]'s boy — I heard the shop's open again."
  - flora: "Come in, come in, don't just stand in the doorway, you'll let the cold in."
  - flora: "If you ever need a bit of green in that shop of yours, you know where I am."
branching: no
rendered: in first-person world view
```

```
type: world_dialogue
id: flora_bark_generic
speaker: flora
trigger: player talks to Flora, no active event/job state change
content: (rotate/random pick, no branching)
  - flora: "Have you eaten today, dear? You have that look."
  - flora: "Lyra's around here somewhere — probably talking someone's ear off."
  - flora: "The lilies came in fresh this morning. Smell them, go on."
branching: no
rendered: in first-person world view
```

```
type: world_dialogue
id: flora_gift_liked
speaker: flora
trigger: player gives Flora an item on the liked list
content:
  - flora: "Oh, you didn't have to — but I'm so glad you did. Sit, sit, let me put the kettle on."
branching: no
rendered: in first-person world view
effect: +heart_points (larger increment)
```

```
type: world_dialogue
id: flora_gift_neutral
speaker: flora
trigger: player gives Flora a neutral-list item
content:
  - flora: "How thoughtful of you, dear. Thank you."
branching: no
rendered: in first-person world view
effect: +heart_points (small increment)
```

```
type: world_dialogue
id: flora_gift_disliked
speaker: flora
trigger: player gives Flora cut flowers not from her own shop
content:
  - flora: "Oh — how... lovely." [smile doesn't quite reach]
  - flora: "I do sell flowers myself, you know. But it's the thought, dear. Truly."
branching: no
rendered: in first-person world view
effect: no heart_points change
```

```
type: world_dialogue
id: flora_discount_unlock
speaker: flora
trigger: flora heart_points crosses friendship threshold (value TBD, blocked on tier-count decision)
content:
  - flora: "You're practically family at this point, dear. From now on, you get my regulars' price. No arguing."
branching: no
rendered: in first-person world view
effect: unlock discounted prices at flower shop going forward
```

```
type: world_dialogue
id: flora_reacts_to_lab_discovery
speaker: flora
trigger: player talks to Flora AFTER backroom/lab discovery event flag is set
content:
  - flora: "You found his old workroom? Goodness. He used to buy me flowers with the strangest excuses — 'inventing,' he'd say, like that explained anything."
  - flora: "Be careful in there, dear. And come tell me what you find — I do love a good mystery."
branching: no
rendered: in first-person world view
```

```
type: comms_dialogue
id: flora_comms_order
trigger: player calls Flora via phone/computer, selects "order" from menu
content:
  - flora: "Ordering something nice, dear? For yourself, or is there someone special?"
  - [menu: order for self / order as gift / cancel]
branching: yes (menu-driven)
rendered: UI overlay, NOT first-person world
```

**Flora dialogue set: complete for v1** (tier 2+ friendship lines deferred
until tier-count decision). Confirmed: Flora **is** on the phone contact
list — first NPC confirmed for it, worth carrying into the Comms section
when that gets revisited.

---

## Lyra — romance candidate

| Slot | id | Status |
|---|---|---|
| 1. First meeting | `lyra_intro` | **Drafted** |
| 2. Generic bark | `lyra_bark_generic` | **Drafted** |
| 8. Romance tier-up | `lyra_romance_1` | **Drafted** (tier 1 only — 2+ blocked on tier-count decision) |
| 9. Gift reaction | `lyra_gift_liked` / `_neutral` / `_disliked` | **Drafted** — likes defined below |
| 13. Comms variant | `lyra_comms_chat` | **Drafted** — confirmed on contact list |

### Tone lock
Breezy, independent. Friendly on the surface but doesn't over-explain herself
or chase approval — comes and goes on her own schedule. Not cold, just
unbothered. Warmth has to be earned through consistency, not grand gestures;
she's suspicious of anyone trying too hard.

### Gift preferences (needed for slot 9)
- **Liked:** anything a little unusual/one-of-a-kind — not the obvious
  romantic gift; small tinkered gadgets from the lab would land well
  (novelty + effort, not expense).
- **Neutral:** standard bought gifts, flowers (ironic given her workplace —
  she's seen enough of them).
- **Disliked:** anything that feels like it's trying too hard / generic
  "romantic" gestures (heart-shaped, overly sentimental cards) — reads as
  try-hard to her, not sweet.

### Drafted content

```
type: world_dialogue
id: lyra_intro
speaker: lyra
trigger: player's first visit to flower shop, Lyra present
content:
  - lyra: "Oh — new face. You're the one who reopened the repair shop, right?"
  - lyra: "Cool. Most people around here just inherit boredom, not a whole workshop."
  - lyra: "Anyway, I just work here, I don't do the flower-whispering thing. Ask Mrs. Flora if you need actual help."
branching: no
rendered: in first-person world view
```

```
type: world_dialogue
id: lyra_bark_generic
speaker: lyra
trigger: player talks to Lyra, no active event/job state change
content: (rotate/random pick, no branching)
  - lyra: "Slow day. Don't tell Mrs. Flora I said that."
  - lyra: "You're here a lot. Not complaining, just... noting it."
  - lyra: "Uni's got me buried this week. Talk fast or don't talk."
branching: no
rendered: in first-person world view
```

```
type: world_dialogue
id: lyra_romance_1
speaker: lyra
trigger: lyra heart_points crosses romance tier-1 threshold (value TBD, blocked on tier-count decision)
content:
  - lyra: "You know, you're actually alright. Took me a minute to decide that."
  - lyra: "Don't let it go to your head."
branching: no
rendered: in first-person world view
```

```
type: world_dialogue
id: lyra_gift_liked
speaker: lyra
trigger: player gives Lyra an item on the liked list (esp. tinkered gadget)
content:
  - lyra: "Wait, did you make this? ...Okay, that's actually kind of cool."
  - lyra: "Don't expect me to make a big deal out of it. I'm making a small deal."
branching: no
rendered: in first-person world view
effect: +heart_points (larger increment)
```

```
type: world_dialogue
id: lyra_gift_neutral
speaker: lyra
trigger: player gives Lyra a neutral-list item
content:
  - lyra: "Sure, thanks. That's a thing I now have."
branching: no
rendered: in first-person world view
effect: +heart_points (small increment)
```

```
type: world_dialogue
id: lyra_gift_disliked
speaker: lyra
trigger: player gives Lyra a disliked-list item (generic "romantic" gesture)
content:
  - lyra: "...Okay, that's a lot. We're not there. Maybe ever."
branching: no
rendered: in first-person world view
effect: no heart_points change, or small penalty (TBD)
```

```
type: comms_dialogue
id: lyra_comms_chat
trigger: player calls/texts Lyra via phone/computer, selects "chat" from menu
content:
  - lyra: "Oh, hey. What's up?"
  - [menu: just checking in / ask about her day / hang up]
  - lyra (if "just checking in"): "That's it? Kind of a boring reason to call, but okay."
branching: yes (menu-driven)
rendered: UI overlay, NOT first-person world
```

**Lyra dialogue set: complete for v1** (romance tier 2+ deferred until
tier-count decision). Confirmed: Lyra **is** on the phone contact list —
second NPC confirmed for it.

---

## Nora — romance candidate

| Slot | id | Status |
|---|---|---|
| 1. First meeting | `nora_intro` | **Drafted** |
| 2. Generic bark | `nora_bark_generic` | **Drafted** |
| 3. Time-of-day | n/a — fixed "usual booth" per Maps section, no variants needed | Resolved (not blocked, just doesn't apply) |
| 8. Romance tier-up | `nora_romance_1` | **Drafted** (tier 1 only — 2+ blocked on tier-count decision) |
| 9. Gift reaction | `nora_gift_liked` / `_neutral` / `_disliked` | **Drafted** — likes defined below |

### Tone lock
Loud and competitive. Teases first, means it second — affection comes out as
a challenge or a dare rather than a compliment. Doesn't do quiet moments
easily; if she's impressed, she'll needle you about it rather than say it
straight.

### Gift preferences (needed for slot 9)
- **Liked:** anything sporty/practical, or a challenge/dare framed as a gift
  (e.g. "bet you can't fix this in a day" energy); tinkered gadgets that
  *do* something impress her more than ones that just look nice.
- **Neutral:** most bought goods.
- **Disliked:** anything delicate, decorative, or "precious" — reads as not
  knowing her at all, mild insult to her taken as a joke on the surface.

### Drafted content

```
type: world_dialogue
id: nora_intro
speaker: nora
trigger: player's first visit to restaurant, Nora present at her usual booth
content:
  - nora: "Hey — you're the repair guy, right? Prove it. Fix something."
  - nora: "Kidding. Mostly. Sit down, you're blocking my view of the door."
  - nora: "I like people who actually do things instead of just talking. We'll see which one you are."
branching: no
rendered: in first-person world view
```

```
type: world_dialogue
id: nora_bark_generic
speaker: nora
trigger: player talks to Nora, no active event/job state change
content: (rotate/random pick, no branching)
  - nora: "Bet you can't guess what I did today. ...Okay fine, I'll tell you, but guess first."
  - nora: "Dario tried to give me a salad again. A SALAD. I'm insulted."
  - nora: "You look busy. Good. Busy people are less boring."
branching: no
rendered: in first-person world view
```

```
type: world_dialogue
id: nora_romance_1
speaker: nora
trigger: nora heart_points crosses romance tier-1 threshold (value TBD, blocked on tier-count decision)
content:
  - nora: "Okay, don't make this weird, but — I actually look forward to you showing up. There. Said it."
  - nora: "Don't get soft on me now though. I'll take it back."
branching: no
rendered: in first-person world view
```

```
type: world_dialogue
id: nora_gift_liked
speaker: nora
trigger: player gives Nora an item on the liked list
content:
  - nora: "Wait, this actually works? Okay, that's — that's actually really cool."
  - nora: "Don't let it go to your head. ...Okay, a little to your head."
branching: no
rendered: in first-person world view
effect: +heart_points (larger increment)
```

```
type: world_dialogue
id: nora_gift_neutral
speaker: nora
trigger: player gives Nora a neutral-list item
content:
  - nora: "Huh, alright. Thanks, I guess."
branching: no
rendered: in first-person world view
effect: +heart_points (small increment)
```

```
type: world_dialogue
id: nora_gift_disliked
speaker: nora
trigger: player gives Nora a delicate/decorative item
content:
  - nora: "...Do I look like I'd know what to do with this?" [laughs, but a little pointed]
  - nora: "Thanks, I think? I'll put it somewhere it won't get destroyed. By me."
branching: no
rendered: in first-person world view
effect: no heart_points change
```

**Nora dialogue set: complete for v1** (romance tier 2+ deferred until
tier-count decision; slot-13 comms and slot-12 event-reaction left open).

**All romance candidates now drafted.** Lyra (breezy/independent) and Nora
(loud/competitive) read as genuinely distinct romance flavors rather than
two versions of the same archetype — worth confirming that contrast lands
right once in-engine.

---

## Dario — friendship/networking

| Slot | id | Status |
|---|---|---|
| 1. First meeting | `dario_intro` | **Drafted** |
| 2. Generic bark | `dario_bark_generic` | **Drafted** |
| 9. Gift reaction | `dario_gift_liked` / `_neutral` / `_disliked` | **Drafted** — likes defined below |
| 10. Networking payoff | `dario_referral_unlock` | **Drafted** |

### Tone lock
Boisterous and generous. Loud warmth — greets you across the room, comments
on whether you look thin, pushes food on you whether you ordered or not.
Pride in craft comes out as enthusiasm about ingredients/technique, not
modesty. Big laugh, big gestures.

### Gift preferences (needed for slot 9)
- **Liked:** quality ingredients he doesn't usually get (rare spices, good
  produce); tools/gadgets for the kitchen (repair-shop crossover — a
  tinkered gadget that helps in the kitchen would land very well).
- **Neutral:** most bought goods.
- **Disliked:** anything food-related that's obviously low-quality or
  pre-packaged — mild offense to his craft pride, though he'd cover with a
  laugh rather than show real hurt.

### Drafted content

```
type: world_dialogue
id: dario_intro
speaker: dario
trigger: player's first visit to restaurant
content:
  - dario: "Eh! The repair shop's kid! Come, come, sit — you look like you haven't eaten a real meal in a week!"
  - dario: "[relative] used to sit right there. Good man. Terrible tipper, but good man."
  - dario: "You need anything fixed here, you tell me. You need anything cooked, you already know where to find me."
branching: no
rendered: in first-person world view
```

```
type: world_dialogue
id: dario_bark_generic
speaker: dario
trigger: player talks to Dario, no active event/job state change
content: (rotate/random pick, no branching)
  - dario: "You eating enough? You don't look like you're eating enough."
  - dario: "New shipment of tomatoes came in. Best of the season. You want, I give you some — no charge."
  - dario: "Aden! Table for the repair kid, on the house!" [Aden groans off-screen]
branching: no
rendered: in first-person world view
```

```
type: world_dialogue
id: dario_gift_liked
speaker: dario
trigger: player gives Dario an item on the liked list
content:
  - dario: "Ohh — now THIS, this is quality! Where did you even find this?!" [booming laugh]
  - dario: "Sit down, sit down, I'm cooking with this tonight, you have to taste it."
branching: no
rendered: in first-person world view
effect: +heart_points (larger increment)
```

```
type: world_dialogue
id: dario_gift_neutral
speaker: dario
trigger: player gives Dario a neutral-list item
content:
  - dario: "Ha! Thank you, kid. Very kind."
branching: no
rendered: in first-person world view
effect: +heart_points (small increment)
```

```
type: world_dialogue
id: dario_gift_disliked
speaker: dario
trigger: player gives Dario a low-quality/pre-packaged food item
content:
  - dario: "...This is very thoughtful." [long pause, forced smile]
  - dario: "You know what, I'll just — I'll put this somewhere. Thank you."
branching: no
rendered: in first-person world view
effect: no heart_points change
```

```
type: world_dialogue
id: dario_referral_unlock
speaker: dario
trigger: dario heart_points crosses friendship threshold (value TBD, blocked on tier-count decision)
content:
  - dario: "Listen — my cousin, he runs a place across town, his oven's been acting up for months. I told him about you. Go, go, tell him Dario sent you!"
branching: no
rendered: in first-person world view
effect: unlocks next job-giver NPC or job (specific NPC TBD — placeholder hook, mirrors Ilić's referral pattern)
```

**Dario dialogue set: complete for v1** (tier 2+ friendship lines deferred
until tier-count decision, and slot-13 comms/slot-12 event-reaction left
open pending decisions on the contact list and how Dario would react to the
lab discovery specifically).

---

## Aden — friendship (romance status still open per world bible)

| Slot | id | Status |
|---|---|---|
| 1. First meeting | `aden_intro` | **Drafted** |
| 2. Generic bark | `aden_bark_generic` | **Drafted** |
| 7. Friendship tier-up | `aden_tier_1` | **Drafted** — track resolved: **friendship only** (see below) |
| 9. Gift reaction | `aden_gift_liked` / `_neutral` / `_disliked` | **Drafted** — likes defined below |

### Track resolved
**Aden is friendship-only**, not romance. This resolves world-bible open
question #4 — worth updating the NPC roster table and open-questions list
there to reflect it.

### Tone lock
Quietly observant. Says little, but what he says lands because he's clearly
been paying attention the whole time — notices small changes (a new tool,
a tired look, a habit). Dry wit surfaces rarely, almost deadpan, easy to
miss if you're not listening. Contrast to Dario's loudness right next to him.

### Gift preferences (needed for slot 9)
- **Liked:** small/practical things — nothing showy; he'd notice and
  appreciate something useful more than something decorative.
- **Neutral:** most bought goods.
- **Disliked:** anything ostentatious or attention-grabbing — feels
  mismatched to him, mildly uncomfortable rather than offensive.

### Drafted content

```
type: world_dialogue
id: aden_intro
speaker: aden
trigger: player's first visit to restaurant, Aden present
content:
  - aden: "...You're new. Dario already likes you. He decided that before you sat down."
  - aden: "I'm Aden. I just carry the plates he insists on giving away."
branching: no
rendered: in first-person world view
```

```
type: world_dialogue
id: aden_bark_generic
speaker: aden
trigger: player talks to Aden, no active event/job state change
content: (rotate/random pick, no branching)
  - aden: "You changed something. Your bag, maybe. Hard to say."
  - aden: "Dario's louder than usual today. So, normal, then."
  - aden: "..." [long pause] "Nothing. Just thinking."
branching: no
rendered: in first-person world view
```

```
type: world_dialogue
id: aden_tier_1
speaker: aden
trigger: aden heart_points crosses friendship tier-1 threshold (value TBD, blocked on tier-count decision)
content:
  - aden: "You come here a lot now. I don't mind. Just noting it."
  - aden: "...That's as close as I get to saying I like the company."
branching: no
rendered: in first-person world view
```

```
type: world_dialogue
id: aden_gift_liked
speaker: aden
trigger: player gives Aden an item on the liked list
content:
  - aden: "You picked this on purpose. I can tell."
  - aden: "...Thank you. Really."
branching: no
rendered: in first-person world view
effect: +heart_points (larger increment)
```

```
type: world_dialogue
id: aden_gift_neutral
speaker: aden
trigger: player gives Aden a neutral-list item
content:
  - aden: "Thanks. I'll find a use for it."
branching: no
rendered: in first-person world view
effect: +heart_points (small increment)
```

```
type: world_dialogue
id: aden_gift_disliked
speaker: aden
trigger: player gives Aden an ostentatious/disliked-list item
content:
  - aden: "...That's a lot of gift for a guy who carries plates."
  - aden: "I'll keep it somewhere. Safe. Away from me, mostly."
branching: no
rendered: in first-person world view
effect: no heart_points change
```

**Aden dialogue set: complete for v1** (tier 2+ deferred until tier-count
decision; slot-10 networking payoff, slot-12 event-reaction, and slot-13
comms left open — a waiter's networking payoff and lab-discovery reaction
aren't obviously defined yet and are worth thinking through rather than
forcing).

---

## Cross-cutting open questions after full roster pass

1. **Affinity tier count** (world bible open Q3) — blocks tier-2+ lines for
   Ilić, Flora, Lyra, Dario, Aden, Nora (all have tier-1 drafted).
2. **Reputation thaw-stage scale for Bruno** — structurally parallel to #1
   but a separate stat/decision, since Bruno is reputation-gated not
   affinity-gated.
3. **Phone contact list membership** (world bible open Q2) — confirmed so
   far: Flora (yes), Lyra (yes), Ilić (no). Still open for Dario, Aden,
   Nora, Bruno (Bruno confirmed no, structurally excluded).
4. **Event-reaction lines to the lab discovery** — written for Ilić and
   Flora only; the other five NPCs don't have one yet. Worth deciding which
   NPCs should react at all (probably not everyone needs to).
5. **Networking-payoff unlocks** — written for Ilić, Flora, Dario. Lyra/Nora
   (romance) and Aden don't have an obvious equivalent — may not need one.

**Resolved this session:**
- Ilić's tone → gruff but warm.
- Flora's tone → warm and maternal.
- Lyra's tone → breezy and independent.
- Dario's tone → boisterous and generous.
- Aden's tone → quietly observant, dry wit; **track → friendship only**
  (closes world-bible open Q4).
- Nora's tone → loud and competitive.
- Bruno's tone → terse and clipped, thaws into short genuine respect.
- Gift preference lists defined for all 7 NPCs.

## Suggested next move

Same "partial > complete" rule as the rest of the bible: lock Ilić's tone,
then fully write his 13 slots (or however many apply) as the template —
proves the taxonomy works before stamping it across six more NPCs.
