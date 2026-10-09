# Save persistence fixture composition — 2026-10-09

Prepared against the isolated support85/Rete63 checkpoint; no Godot parser,
build, test, conversion or runtime was launched by this integrator.

The older `save_persistence.gd` fixture bypassed the new explicit picker, treated
gold as ordinary item0, assumed native schema1, and supplied detached mocks
without the current semantic player binding or native checkpoint Resources.

The actual Manor stages now choose the original import for the initial write
and the native checkpoint for reload through the production picker buttons.
They assert that `game_session.selected_save` retains the requested identity
and state. Corrupt, unsupported and extra native state remains rejected; its
disabled row and reason are checked without hiding a valid original choice.
The fixture changes catalogue member21 directly and checks separate native
gold20→0 through the full inventory snapshot and a fresh process. No ordinary
item0 is admitted. The gate's source-pose+2.6X, mana consumption, unavailable
sentinel, optional flags and preservation of a valid slot after bad writes
remain covered.

Optional-field coverage reuses `actor_world_save_fixture.gd`: production
inventory, life, armor/facade, semantic actor bindings and real floor contact.
The current production writer persists the native schema given by
`SAVE_STATE.SCHEMA`. Legacy schema1 omission is retained as component-overlay
coverage and is explicitly rejected as a complete Load Game checkpoint.
Unavailable DSG147 rejection checks different saved mana and gate values to
detect mutation before failure. Rust proof markers match the revised fixture;
existing test IDs, prerequisites and deadlines remain present.

Pending Luna syntax/contract execution: the optional-field test requires only
these maintained fixtures. The three actual Manor stages require the corrected
generated game and remain separate from the passing save-selection harness.
The existing whole-test100s deadline is unchanged; final loading costs must fit
it or be addressed explicitly by the test owner. No acceptance pass is claimed.
