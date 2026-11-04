# Multi-answer dialogue items (question conversations), build MaiDFXvr_bleu.exe

Tags: [P] proven from decompilation/disassembly, [I] inferred. Addresses are functions in
reference/decompilation/functions/<addr>_*.c (Ghidra line numbers in parentheses).
Item = DAT_00723b04 (`DLG_fn_xGetCurrentPointer` 0056d850). Ghidra loses some call arguments
(e.g. 0056e360); disassembly (`objdump`) was used where noted.

## Records
Item: +0 answer count, +1 selected index, +2 box style, +4 answer-pointer array, +0xc line list head,
+0x10 owner object, +0x14 mode (1 = body pages, 0 = answer page), +0x18 name, +0x1c current page line,
+0x20/+0x24 state refs, +0x28/+0x2c channel lists, +0x30 item speech/sound. [P] 0056f4e0 parser
(ItemAnswer/ItemText/ItemBox/ItemSound/...), 0056c700 InitDraw.
Answer: +0 / +4 inventory gobject/tobject (cost), +8 one-shot ("AnswerDelete"), +0xc font OFF (default
DAT_00735060), +0x10 font ON (default DAT_00735064), +0x18 next item ("AnswerNext" -> `<dlg>_DialogItem_<x>`),
+0x1c comportment ("AnswerComportment", a script Comport section), +0x20 selected flag, +0x28 text line list
("AnswerText"), +0x2c/+0x30 two "AnswerState" refs, +0x34/+0x38 activate/deactivate channel lists, +0x3c speech
("AnswerSound"), +0x40 count / +0x44 array of {byte code, item*} ("AnswerBad"), +0x48 gold, +0x4c icon (-2 none).
[P] 0056fec0 (AnswerSection parser, lines ~60-1080).

## 1. Data layout
- Body pages = the item's own ItemText lines (item+0xc list), paged in groups of rows-per-style
  (table 0060bcf8/0060bd28: style0=2, style1=6, style2=10, style3=2, style4=2, style5=6, style6=10, style8/9=10) [P].
- Each answer owns a separate AnswerText line list (+0x28): "Yes"/"No" are NOT body lines; they are text of
  answer 0 and answer 1 [P] (0056dd50 iterates answers and their +0x28 lists).
- Speech: one per item (item+0x30, played on item owner at first draw, 0056c2d0 l.42-44 -> 0041e8a0), plus an optional
  one PER ANSWER (+0x3c). The answer speech is played on the MainActor holder (443460(443470())) when that answer is
  SELECTED: 0056e3b0 l.30 -> 0056e360 -> 0041e8a0(MainActor, answer+0x3c) [P, disasm 56e360-56e37d].
  Item speech is NOT answer 0's speech. Selected-answer speech starts as part of Select, before/with the branch.
- +0x08 one-shot [P]: after selection the answer is removed from the item's array (shifted, count--, 0056e3b0
  l.34-45); selected index reset to 0 and answers[0].selected=1. (Persistent in this dialog object until unloaded.)
- +0x18 branch [P]: on success the next item (answer+0x18) becomes pbVar6; if non-null `DLG_fn_vInitDraw(next)`
  (0056c700) restarts the dialogue on it (chain, same box, fade not reset). Null -> dialogue closes.
- +0x44 mapping [P]: array of {code, item*} (AnswerBad). Used only when the cost check (0056e2c0) returns a code
  other than -1 (e.g. not enough gold / inventory item missing). Count 1 -> that item, else lookup by code, fallback to
  the first entry; if no mapping, stay on the SAME item (`pbVar6 = pbVar1`). Yes/No quests without gold/inventory
  never take this path: 0056e2c0 returns -1 when answer+0x48 == 0 and gobject/tobject unset (FUN_0056e0e0 returns 1
  for gold 0) [P for gold; inventory branch 0056e1e0/0056e110 not traced].

## 2. Drawing
- Answers appear on the SAME page as the last body page, appended after its last line, no extra input [P]:
  0056cb10 (page drawer) when the list ends (`*param_1==0`) and answers[0]+0x28 != 0 sets item+0x14 = 0 (mode: answers)
  immediately (l.~44-50); the typewriter path 0056cc70 (l.~70-78) does the same. In mode 0 Draw (0056c2d0 else
  branch, l.~108-148) finds the start of the LAST page (every (n-1)%rows==0 node), redraws it with 0056cb10 (which
  returns the number of lines drawn, uVar4), then 0056dd50(item, uVar4) draws all answers.
- Rows: first answer line is on row = (#lines drawn on that last page); every answer line increments the row; answers
  are contiguous, 1 row per line, across answers [P] (0056dd50 `iVar6`). Overflow when the last page is full (6 rows
  in style 1) is not guarded [I]; authors must keep the last page short. Question text = last line(s) of the body.
- Row geometry [P] 0056c8e0: world Y = originY3D - row*0.3 (+ line anchor offset); originY3D = 3.375 -
  (border height + boxY px)*0.0140625 (0056d770). X = originX3D + line offset (+0.5 units for answer rows, i.e. answers
  are indented ~35.6 px at 640 wide; style 3 uses -4.45 instead). 0.0140625 = 9/640 units per px, so row step = 21.33 px.
- Selected answer [P] (0056dd50 l.~50-80): drawn first four times at (+-0.01, +-0.01) units in forced font
  DAT_00735068 (3rd InitFont font), then drawn normally in answer+0xc (DAT_00735060 unless overridden) on top.
  Unselected answers: only the normal draw. So colour is FONT-driven, no colour constants: selected = font[2] halo
  copies (+ whatever font answer+0xc is), unselected = font[0]. The observed "blue selected, white unselected" implies
  font[2]/answer font data; answer+0x10 (FontON) is parsed but not referenced by 0056dd50 [P]. Open: read the
  .dlg root fonts (snapshot words 8,12,16 = DAT_00735060/64/68) and the raw answer +0xc/+0x10 to know exact fonts.
  (Same halo idiom exists for emphasis font DAT_00735064 in body text, 0056db30 l.55-70.)
- Bullet (orange arrow) [P] 0056c8e0 l.8-12: when drawing a selected answer's lines (flag DAT_00723b1c=1) stencil #5
  ("DLG_bullet", 0056bd30) is placed at x = boxX + 40, y = row*20.5 + boxY + 35 (screen px, sprite top-left, via
  0056bfb0(5,...) + 0047c8e0), once per line of the selected answer (row = that line's row). Note 20.5 px step vs 21.33 px text.
- Cost icon (+0x4c != -2) also drawn with stencil 6 at x boxX+162..217; ignore for Yes/No.
- Continuation red_arrow hidden on the answer page [P]: 0056d5c0 (arrow) is reached only from mode 1 when page complete
  and `DAT_00723af0 == 0` (not final page) (0056c2d0 l.~99-103); the mode-0 branch never calls it.

## 3. ScrollDialog(direction) = DLG_fn_vScroll 0056f3a0 (AI proc 326, handler 543cbb)
- Returns immediately if mode == 1 (body pages) or answer count < 2 [P]. So it only acts on the answer page.
- Wraps both ways [P]: dir<=0 -> idx-1, wraps 0xff -> count-1; dir>0 -> (idx+1)%count.
- Sound [P]: FUN_004ec880(-1, DAT_00754400, DAT_00723b2c[+8]) (sound table loaded from fix SoundEffects in
  0056bd30). Plays only when the scroll actually moves (past both guards), even with count==2.
- Moves selected flag (+0x20) 0 -> new. After the handler returns, disasm 543ce0-543e6f: if the new selected answer's
  +0x2c (first AnswerState ref) is non-null, calls FUN_00435250(ctx, ref, 0) (change owner's state/comport) then
  FUN_005223b0(2) [I: used for highlight animation of the speaker; not needed for Yes/No].

## 4. SelectDialog = DLG_fn_vSelect 0056c690 (AI proc 327, handler 543d07)
- No-op unless fade-in finished (DAT_00723b18 != 0). Dispatches on mode [P]:
  - Mode 1 -> FUN_0056e510 (page advance): skip rules for lines flagged &2, advances `rows` lines through the list; at
    list end: if a speech is still playing sets DAT_00723b44 (wait), else item+0x14 = 0. THEN, mode 0 and answers[sel]
    has NO text lines -> auto resolution: if answer+0x1c == 0 then next = answer+0x18 (returned, chain/close), else
    the pending callback is stored and the dialogue closes (this is the existing single-answer path) [P l.~90-110].
  - Mode 0 -> FUN_0056e3b0 (answer pick), described below.
- FUN_0056e3b0 [P]: cost check 0056e2c0 -> -1 success. Then: `DLG_fn_vGetSetCurrentPointer(item+0x10,
  answer+0x1c)` stores {owner, comport} as the PENDING CALLBACK (globals DAT_00723b38/3c); apply activate/deactivate
  channel lists (0056e800); stop/start: clear speech selected flag, play answer speech (+0x3c, on MainActor); remove
  one-shot answer; reset selection to answer 0; result item = answer+0x18 (branch) or mapping (cost failure).
  Result non-null -> InitDraw(result) (chain); null -> `*param_1=0` -> caller 0056c2d0/handler 543d58 calls
  DLG_fn_vEndDraw(1) (fade out, then close, DAT_00723b04=0).
- Select button sound: 543d46 plays DAT_0075ec64 click (4ecc90) when mode==0 only.
- Also after Select, disasm 543d80-543e6f: if the PREVIOUSLY selected answer's +0x30 ref is non-null, FUN_00435250(ctx,
  ref, 0) changes state [I same family as +0x2c].
- Pending callback consumption [P]: Cond 120 (535250 case 0x78): requires 0056c8c0 IsDialogOver (alpha<=0.1, 0056da60) and
  MainActor holder free (41e950); then ChangeComport(owner, comport, !flag) via 535360/5216e0, clears pending, true.
  Cond 121: same gate, true without consuming. Proc 329/330 (543e8f) consume the pending pair explicitly. Matches ui.md.
  So the answer's callback is stored at selection time and fires only after the close fade completes.

## 5. Timing
- Initial selection: answer 0, answers[0]+0x20 = 1, item+1 = 0 [P] (0056c700 l.~16-26).
- Mode 0 (answer page) has NO timer: 0056c2d0 else-branch contains no speech deadline check; waits for Select [P].
- Mode 1 deadlines: line+0x18 (end_seconds, default 0x7f7fc99e = 3.4e38 = never, 0056e040) compared with item speech
  playback position (`_DAT_00723b14 < pos*1.5258789e-5` -> Select), or on final page with exhausted speech (`DAT_00723af0==1 &&
  DAT_00723b24` and finite deadline) -> Select + EndDraw (l.~64-80). Risk [I]: 0056cb10 flips item to mode 0 as soon as the
  last page is drawn, so a finite deadline on the final (question) page would auto-select answer 0 via the
  same Select; question items must have the 3.4e38 default. Check the actual line end_seconds of wellet_* items.
- Draw also waits for MainActor speech holder to be free before starting (0056c2d0 l.30-33) and fades in 0.1/tick (0056d860).

## 6. Misc / not pursued
- Line flags (+0xc): bit0 = draw immediately (skip typewriter reveal), bit1 = no skip/hold in 0056e510. Typewriter reveal only when
  item has no speech, mode 1, not fast-forward (0056c2d0 l.~84-90 -> 0056cc70, 1 char / 2 ms-tick step). Not verified for monastery plaque.

## Open questions
1. Exact fonts for answer+0xc/+0x10 and root font[2] (colour of "blue"): need .dlg/.rtd bytes.
2. Inventory-based failure codes (0056e110/0056e1e0) irrelevant for Yes/No; not traced.
3. Whether author data ever gives a finite end_seconds on a question page (see 5).
4. 0056e510 DAT_00723b44 semantics (waiting for speech end before entering answers) read only partially.

## Port status and take evidence (2026-11-04)
- Implemented in the importer, Godot export and ui/dialogue.gd + screen.gd; regression
  godot/tests/dialogue_answers.gd on the generated Wellet items.
- Take stephen-linear-brigand-20261102, 583-591 s (30 Hz): pages, then "Would you like to?"
  with Yes/No on the next rows; Down moves the bullet; No cuts the box at once and the next
  item (wellet_refus) fades in from zero over about 6 frames before its text. So a chained
  InitDraw does restart the fade; the "fade not reset" reading above is corrected.
- Selected-answer halo in the take: ring about (21, 21, 162) over a (42, 39, 54) background,
  matching the third root font (0, 0, 0.9) blended with premultiplied alpha. The dialogue
  atlas stores premultiplied edge colour (RGB ~ alpha), so source span labels now use a
  premultiplied CanvasItemMaterial.
- Item 0056c2d0 deadlines: no question page in Manoir/Brigand has a finite deadline.
- Brigand Zatila 4/5 put 5 body rows plus 2 answers on the last page (7 rows); drawn as the
  source does, not compared with an original capture.
