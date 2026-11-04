# Static traces: ModelCast field receivers and directional turns (2026-11-04)

Static reads of the pinned executable (`reference/hype_patch/MaiDFXvr_bleu.exe`).
The clean rewrites are in hype-decomp 88ccd90:
- `src/ai/actor_field_operators.c`;
- `src/ai/dynamics_event_procedures.c`;
- `src/dynamics/mechanics_event_dispatch.c`.

## Actor fields through Operator_ModelCast (operator 2-25)

The operator table at 0x005fff44 names operator 25 Operator_ModelCast
(handler 00525990) and operator 12 Operator_Dot (00525010).

- **ModelCast:** evaluates its first operand, a category-39 Model leaf (value
  tag 41), into the result and never compares it.
  - It then evaluates the actor operand to a SuperObject.
  - It returns that SuperObject's Perso, tagged 12, or 0 when the
    SuperObject's type word is not 4.
- **Dot:** reads the node after its receiver from the actor's own Brain:
  - category 11 is a designer field;
  - category 9 is an engine field.

  The model therefore only says, at authoring time, which model's field
  numbering applies.

The six brigand sites read `((Model)dsg_2).field` in models 927 and 1060. The
receiver is an actor held in a designer variable. Typing them needs two things
in the converter:
- a kind-39 binding to a model;
- a field read on a runtime actor.

Kind 39 has no binding yet and its live value is unchecked, so this is not
ported. The converter's message names the shape: kind 2 parameter 25 and
kind 39.

## Directional turns (procedures 36, 37, 38, 43)

00540220 serves procedures 33 to 51 and 223. It posts one event to the
receiver's dynamics (Perso +8) through 0052eeb0:
- TurnLeft: event 3;
- TurnRight: 4;
- TurnUp: 1;
- TurnDown: 2.

The event table (dynamics +0xfc) holds up to 8 distinct events. 004440e0 hands
each one to 00443e00. That function calls the update of the receiver's
identity-card type: the table at 0x00757324, 8 bytes per type, filled by
00443540. The update takes six arguments, from the PE pushes: block,
SuperObject, state, card, frame seconds and event.

All 384 turn sites are in model 124, used only by iHypeZat in ciel1 to 4. Its
only mechanics card (shared 27270) is type 19, whose update is 0057e110 (also
used for type 20). Its switch subtracts 3 from the event and bounds the result
at 10 (PE 57e175):
- **Events 3 and 4 (TurnLeft, TurnRight):** set +0xe0 to 2, build a rotation
  of ±(+0x39c × +0x3a8) about local Z, and compose it into the state's angular
  term at +0x1d8. They stay stand-ins: the port has no type-19 mechanics.
- **Events 1 and 2 (TurnUp, TurnDown):** fall to the default and change
  nothing. 00444a25, called before each update, only recomputes a flag byte.
  The port types these as `TurnVertical`: a no-op while the receiver's card is
  type 19 or 20, and an explicit failure otherwise.

  The one difference left: the source's event uses one of the 8 table slots
  for that frame.
