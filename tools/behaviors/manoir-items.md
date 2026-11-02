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
- [ ] Barrels.
- [x] Sword hits register: the training mannequin reacts and recovers.
- [ ] Damage, death and fall damage.
- [x] Training: Gogoud leads Hype to the mannequins, dialogue plays, sword stage completes with the source spin sequence, Gogoud advances.
- [x] Talking to Gogoud grants the crossbow and 10 bolts, and item messages close.
- [ ] Crossbow stage: bolts fire and clear too soon to register on the mannequin.
- [ ] Magic stage, Gogoud's key and the gate.
- [x] A level exit to an unconverted level returns to the menu with a message. Not reached by play yet.

## Fidelity

- [ ] Timing compared with the original.
- [x] Actor collision: Hype stops at the save lectern where the original does.
- [ ] Physics compared with the original: walk and run distance agree within about 3 percent, jump rise within about 6 percent.
- [ ] Sound mixing compared with the original.
- [ ] Visuals compared with the original: camera framing and world match at the Load start; actor lighting and HUD in progress.
- [ ] Sector AI activation: the original leaves some distant actors idle until nearby; the port runs them all.
