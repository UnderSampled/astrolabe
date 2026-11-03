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

- [ ] Wild boars A and B (I_Wild_Boar_A/B): routes from Func_CloserWP, charge, damage, death.
  They run in the port (profiled at 20 to 50 ms of AI per 2 s each); behavior not compared.
- [ ] Wolves (I_Brigand_PJ_Wolf1/2): run in the port; together about 180 ms of AI per 2 s, the
  largest cost in Brigand. Fights not compared.
- [ ] Bats (I_Brigand_Bat_1 to 6, I_Dummy_Brigand_Bats). Not checked.
- [ ] Dagger thief (I_Brigand_Thief_Dagger_1) and its camera (I_Brigand_Camera_Theif_Cam). Not checked.
- [ ] Enemy health bars: the HUD has three enemy meters (EnemyMeter0 to 2, FUN_0040c720); fill
  and timeout follow the source, triggered by Fight_RemoveAndReadHitPointsEnemy. Not shown in a run yet.
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
