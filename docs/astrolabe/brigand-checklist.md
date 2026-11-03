# Brigand checklist

Kept by hand. The reference is Stephen's take `TestResults/stephen-linear-brigand-20261102`
(894 s; Manoir to 323 s, Brigand from 324 s, a toras round trip at 650 to 660 s, Brigand again
to the end) and its `stephen-notes.md`. Times below are take seconds. Freelook was bound to V
in that take.

## Arrival and saves

- [x] Arrival: the Manoir tunnel exit fades, the loading screen shows real progress, Brigand's
  arrival dialogue plays and Hype gets control (to_brigand.json).
- [x] F5 in Brigand saves; Load Game resumes there without replaying the arrival (brigand_load.json).
  Level actors keep fields, behavior, pose and links (travel save stand-in: one slot, no history).
- [ ] Save in the Brigand forest at the original's save point (take snapshot 000003 at 437 s;
  output-saves/SaveGame2). Not compared.
- [ ] Load the after-race save made after the recording (`after-race-save-post-recording/`).
  Not tried with `astrolabe import-save`; its record key table covers the Manoir only.
- [ ] Arrival hitch: the first frames after the load run at about 2 FPS for half a second.
  Not in actor AI; suspected first-use loading. Open.

## Enemies and combat

- [x] Enemies damage Hype: Cond24..27 (zone-type collision, 0042f980) was a stand-in reading 0,
  so the boar, guard, thief and bat AIs never reached their damage branch. Ported. Boar A
  standing next to Hype: life 50 to 37 in 16 s, hits of 1 to 3 with hit reactions (normal 15)
  and the blue flash. The take's sword bar (1 fps frames, about 2.4 px per life point) shows
  boar and thief hits of 1 to 3 at 368, 400, 474 and 556 to 563 s; the 712 s drop (9) spans
  several hits between frames. Per-hit damage is not comparable exactly: no life in the take's
  samples, and enemy actors are not sampled at all.
- [ ] Wild boars A and B (I_Wild_Boar_A/B): routes from Func_CloserWP, charge, death. Boar A
  dies after three sword hits (normal 9). Charge timing not compared.
- [ ] Wolves (I_Brigand_PJ_Wolf1/2): each wolf's dsg_34 (its partner) is stored as a
  SuperObject and was left null, so the hit rule and the circling behavior crashed on it.
  Resolved: a sword hit now takes wolf 1 from 15 to 10 (Hype's base damage dsg_115 = 5,
  copied to dsg_114 by the swing object fix/708), and it alternates circling and
  attacking (normals 6 and 10). Death and bars not yet run to the end; not compared with
  the take's wolf fight at 366 s.
- [x] Action-return policy 1 before Proc_Turn (100 sites, guards 927 and thieves 1060) and
  before other-actor comport changes: compiled as a rule hold
  (docs/astrolabe/action-return-policies.md). Brigand stand-in sites 344 to 225.
- [x] Route harness: a `chase` step walks to a moving actor with the real controls
  (the `near` teleport left Hype falling, so Space was swallowed).
- [x] Wolf 1 to its death on foot: 15, 10, 5, 0 at Hype's base damage 5, then its death
  behaviors (normals 8, 11, 14). Its bite took Hype 50 to 49 once. Bars and damage against
  the take's 366 s fight not compared frame by frame.
- [x] Common thief (iThievesCommun_Brigand_01, the take's dagger thief): Cond collide-
  with-nothing answered true for an actor the port does not simulate, so its normal 7
  rule held forever and it never entered reflex 1. Fixed (00533d20 answers false without
  a contact report): hits 15, 10, 5, 0 and death.
- [ ] Guard (iGardeCommun_Brigand_01): the sword touches its zone 1, but Hype's attack
  flag (dsg_17, set by the swing object fix/708) never turns on there, so no damage; the
  guard stays in normal 1. Open.
- [ ] Bats: bat 1 stays about 5.7 m away and above reach; its dive is not reproduced. Open.
- [ ] Bats (I_Brigand_Bat_1 to 6, I_Dummy_Brigand_Bats): bat 1 flies above sword reach in
  the port route (normals 2, 1); no hits either way. Not compared.
- [ ] Dagger thief: the take's thief is iThievesCommun_Brigand_01 (model 1060, same spot as the scripted runner I_Brigand_Thief_Dagger_1). Earlier: approaching starts its camera (Hype normals
  48, 28, letterbox); swings did not register afterwards. The original takes thief hits at 474 s.
- [x] Enemy health bars: Fight_RemoveAndReadHitPointsEnemy (boars, bats, guards, thieves)
  refreshes the HUD rows as 0040c4a0/0040c8a0 do: up to three rows, oldest dropped, fill is
  life over maximum, a row expires 5 s after its last hit (both source timeouts are 5000 ms).
  Wild boar A, three sword hits: fill 77, 51, 25, 0 px, death (normal 9), bar gone 5.0 s later.
  Against the take (frame 469): bar size, colours and place match within 2 px; the right-edge
  frame (Fix panel 1) was missing and now shows while any row is active.
- [ ] Respawn: leaving through the toras door and coming back reloads Brigand and respawns the
  mobs (650 to 660 s). Port: travel back into Brigand restores world records, so killed mobs may
  stay dead; needs the source rule.

## Beehives

- [ ] Ten hives (iNid_Abeilles_Brigand1_*) and iDummy_PourNids: shot down with the crossbow,
  each with a HUD meter. Not checked.

## Story and cutscenes

- [ ] Monastery sign (I_Brigand_MonastPlaque) reads its text; the monastery door
  (I_Brigand_Monast_Porte, I_Brigand_Dummy_MonastDoor) refuses to open, with its camera
  (I_Brigand_Dummy_Monast_Camera). Not checked.
- [ ] Triggerable cutscenes: Zatila (I_Brigand_Zatila_1, I_Brigand_Camera_Zatila), toras door
  (I_Brigand_Porte_Toras, I_Brigand_Camera_Porte_Toras), sky (I_Brigand_Dummy_Ciel_*),
  Gogoud (I_Brigand_Dummy_Gogood_*), the door lady, the dragon and logs dummies, the drawbridge.
  The take has cutscene phase 16 at 344 to 355 s and 874 to 878 s. Only the arrival is checked.
- [ ] Toras door loads level 3 and the immediate return comes back to Brigand (650 to 660 s).
  The port returns to the menu for unconverted levels; toras is not converted.

## Wellet's race

- [ ] Wellet (wellet_depart, wellet_refus, wellet_refus2, wellet_plastyksinsuffisants):
  refuses until Hype has enough money, then starts the race. The race is in Brigand, after the
  toras round trip.
- [ ] Race course: 37 markers (iCourse_Brigand1_*), the kid (iKid_CourseBrigand) and the fire
  bottles (iMagic_Bottles_FeuCourse). Not checked.
- [ ] Upgrades sold in Brigand (upgrade_* dialogues): potions, herbs, arrows, plastyks. Not checked.

## AI and data

- [ ] Stand-ins: 383 compiled sites in Brigand's AI. The largest groups are listed by
  `grep -rho 'stand_in("[^"]*"' godot/generated/hype/levels/brigand | sort | uniq -c | sort -rn`.
- [ ] Sector AI activation: the port runs every actor; Brigand's enemy AI is the main frame cost.
- [ ] Frame rate: Brigand runs at 80 to 100 FPS uncapped in the port, against about 230 in the Manoir.
