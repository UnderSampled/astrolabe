# Source dialogue continuation and Hype handoff

Baseline `12ad58ce49d60b10cdcbb80a29c9118a5e9feffa`; isolated branch
`codex/new-game-dialogue-gate-sol-20261009`. No builds, imports, game processes
or runtime tests were launched by this worker. Support85 and Rete63 are
provisional integration stamps shared with other workers' pending changes.

The maintained Dialogue scene lacked a continuation indicator/control. The
useful presentation asset now retains the fifth Fix GPT stencil material's
texture, resolved through RTP and the existing material/texture codec. Source
0056bbe0 writes corner, border, center, side, red_arrow, bullet and the remaining
stencil data in that order. Source0056bd30 and getter0056bfb0 corroborate those
roles. The exporter emits the same texture as an ordinary native Texture2D;
the authored support scene presents it through a TextureButton.

The independently reconstructed0056d5c0 body in source commit
`e9609f36cb82e8e6d2e2b36f66592d59ecd56781` places style3 at wrapped/truncated
viewport fractions608/640 and448/480 without PositionDialog. Admitted framed
styles1/5 use border width*8+10 and side height*4+10 plus the dialog origin.
The sprite's x/y are its minimum/top-left pixel edges. The shared native canvas
continues to own viewport scaling. Full rendered pixel/driver parity is untested.

0056c2d0 owns visibility: the arrow appears on a completed nonfinal page.
It stays hidden during opening, page changes, final pages, closing, cancellation
and MainActor speech-holder drawing waits. Clicking calls the existing request
selection API. The native screen and compiled Hype rules both observe Button_Hit;
the screen now claims a physical press once to prevent duplicate page advances.
Explicit selection without a new input edge retains its API behavior.

Invisible continue input alone does **not** explain an infinite Hype first-reply
stall. Actual `hype_texte1_manoir` is style3 with two lines, final deadline3.88s,
voice V_Manoir26, owner `level:/persos/world/0/28` (statue model294), and completion
Normal5. It has no successor page on which source would draw an arrow. The draw
selects after the owner's playback passes3.88s, or when the exhausted finite
speech page's holder clears. That close sets a pending callback. Hype's
Cond120 consumes it after the close and MainActor speech gates clear, then
switches statue to Normal5. No timeout/skip workaround has been added.

Prepared checks extend the actual nine-resource native UI harness with stencil
binding, opening/visibility, source anchors, one-page click, cancellation and
finite final speech completion without input. The existing synthetic UI harness
checks shared native/compiled input-edge ownership. The source/Rete localization
test retains the new texture through its complete presentation round trip.
The release manifest records these existing bounded checks and corrects the
support staging test's obsolete interface70 name to interface85. `git diff
--check` passed; runtime assertions remain unrun for Kip/Luna to coordinate.

Luna's baseline used main12ad58ce/support84/Rete62. Its passive New Game run
remained at LocationTitle `manoir_txt1`, Hype Normal93/Reflex3 and statue Normal2,
before the reported Hype reply. The outer watchdog ended the process early;
the report's `running` value is a partial receipt, not an active process.
The runtime slot is free and held by Kip. This evidence neither validates these
changes nor attributes the later speaking stall. The next coordinated pass needs
the active item/page/alpha/speech holder, completion Brain/pending flag, Hype
DSG30/56/75 and statue DSG0/action state at the reported stall.
