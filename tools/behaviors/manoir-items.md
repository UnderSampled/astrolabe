## Asset data

- [x] Geometry, textures, collision, sectors, characters, animation and sound convert and load in the real level.
- [ ] Every record type is named. The Manoir conversion reports 378 source diagnostics. Most are AI gaps counted above. Data gaps: animation morph data in 47 animations, spatial acceleration data in 30 geometries, preserved fields in 6 mechanics kinds, and 6 sector domains without codecs.

## Play

- [x] New Game opening plays to handoff, with all 8 dialogues.
- [x] Load Game restores a real save slot through named keys.
- [x] F5 saves, including after the opening.
- [x] The save lectern plays its sequence and saves.
- [x] Walk, run, backstep, turn, jump, sword, crossbow and Fire match the original's sequences.
- [x] Flask pickup collects its item through compiled AI; saved gold stays collected.
- [x] Barrels. Fire at iBaril2 runs the barrel chain to iDummy_PourBarils (stage5_barrel.json). The dummy checks distance at the hit: within 3 units Hype takes armored 10 (life 50 to 49) and is knocked back 1.5, proven by triggering it beside the barrel. Hype out of range takes nothing, and a Fire cast protects him: behavior 12 sets his dsg_32 for the cast and the dummy skips damage while it is set. Not run in the original: the yard is too far from the save for placement to hold, and staged placement fell too. Stephen's recorded route can reach it.
- [x] Sword hits register: the training mannequin reacts and recovers.
- [x] Fall damage and death. Falling off the bridge matches the original on both branches (fall_bridge, fall_near_pont04): behaviors 93, 94, 16, 96, 4, placement by iManoirAll_Dummy_TombePont1 and life 50 to 49 through Global's armored 10. A lethal fall (fall_death, from life 1) matches through 16, 95, 55 with actions 316, 343, then statue, fade, game over image and the menu. Only rendered runs finish the game over: the image's hold waits for a drawn frame.
- [x] Training: Gogoud leads Hype to the mannequins, dialogue plays, sword stage completes with the source spin sequence, Gogoud advances.
- [x] Talking to Gogoud grants the crossbow and 10 bolts, and item messages close.
- [x] Crossbow bolts register on the mannequin: the bolt no longer strikes its own shooter (stand-in `generator-contact`), so the mannequin reacts. Second arm landed and Gogoud advances (stage1_crossbow.json).
- [x] Magic stage: Fire hits both mannequin arms (zones at the arm spheres, not the body), mannequin reaches 4, Gogoud advances to the finimannequin dialogue (stage2_magic.json). Needs the sword drawn first (X). Fixed: Hype's scripted walk afterwards used an actor-relative waypoint the converter left unresolved.
- [x] Gogoud's key: Gogoud activates the key and Hype owns object 21 (stage3_key.json). Fixed: Operator category 2 operation 18 (vector times real, inferred) was unsupported and aborted the key schedule. The key is not auto-selected; select it (route uses select_object) before using it.
- [x] Gate: with object 21 selected, Space within 1.5 of iSerrure_Manoir1 while facing it and standing on its facing side (north) opens iMGrille_Manoir_Pont; Hype walks through and regains control (stage4_gate.json). Standing on the east side shows the serrure_manoir refusal dialogue. The Caverne gate was not tested.
- [x] Exit: reaching the loader dummy at (-79.5, 19) fades out and leaves for the menu (stage6_exit.json by teleport; save_exit.json walks there from the save and is in the gate).
- [x] A level exit to an unconverted level returns to the menu with a message. Not reached by play yet.

- [x] Save restore: after Load Game 224 of 226 saved numeric designer variables on Brain actors equal the live values (route step save_diff). Global dsg_37 differs correctly: the lectern sets it and clears it after the save. Global dsg_17 (saved 0, live true) is unexplained; camera model 437 reads it. The 573 unknown fields are mostly unnamed, not lost: 221 actor variables, 134 dialogue, 110 map UI, 60 map stars.

## Fidelity

- [ ] Timing compared with the original. Intro compared by tools/intro_compare.py: Hype, Gogoud and action sequences, every camera shot, end camera and end positions match. The port finishes about 10s sooner because voice-tied waits run about 1.1 times longer in the original under Wine; the port plays each clip at its true length, and non-voice intervals agree. Whether the stretch also happens on real Windows is unsettled.
- [x] Actor collision: Hype stops at the save lectern where the original does.
- [ ] Physics compared with the original: walk and run distance agree within about 3 percent, jump rise within about 6 percent.
- [ ] Sound mixing compared with the original (tools/intro_loudness.py). Music within 1.5 dB, voices 1.5 to 5 dB quiet at the intro. Camera microphones hold their selected pose, matching the original: an intro line was silent before. Open: between intro lines the background is -51 dB in the port and -42 in the original. Measured by waveform correlation against the Oct 3 audio: music agrees within about 2 dB and the two ambience loops are only about -65 and -73 dB there, so the source of the -42 bed is still unknown.
- [ ] Camera procedures: ChangeAngle, ChangeDistance and the getters are real (disassembly of 00547620 and 00546510); follow distance and height match within about 0.1 settled. Lectern and iCadre room behavior not compared with the original.
- [ ] Actor lighting: actors away from Hype, and Gogoud through the intro, are clearly darker than the original (his face is dark brown, not orange). Not ambient follow, Godot's inverse-square falloff, or vertex shading: each was tried and changed nothing. Likely the compatibility renderer's cap of 8 lights per object, or sector light filtering; needs sector-filtered actor lighting.
- [ ] Visuals compared with the original: all intro camera shots match; camera framing and world match at the Load start; actor lighting and HUD in progress.
- [ ] Sector AI activation: the original leaves some distant actors idle until nearby; the port runs them all. A plain rule (run only sectors in the activity list of Hype's sector) was tried and reverted: it stranded Hype in control lock on the walk to the exit and did not improve the actor match. Needs live evidence of which actors run.
