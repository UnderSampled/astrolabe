# Hype manoir camera research

The visible BDD wrapper currently creates its own `Camera3D` with Godot's
75-degree default FoV, at seven units behind Hype and 3.5 units above him,
looking at an assumed 1.4-unit target height. The original and Godot walk
screenshots from the clean focused run show materially different framing:
Hype is much larger in the original view. The aspect ratios already match
(original 1024×768, Godot 960×720), so distance, orientation and projection
must be measured separately. Earlier source-camera captures in
`material-transparency-evidence.md` measured a 1.5-radian horizontal FoV and
69.8838-degree vertical FoV at 4:3, but their camera pose belongs to a
different viewpoint; it must not be reused as the `manoir` pose.

The source probe used the supplied English save (archive SHA-256
`877576b5867b5ad44b41dc4b72f6b70bc89dbe5e202c86817f2858948a1d634c`,
executable SHA-256
`048ff4f4f0447a1f54a5a73ff588df78b484015c78df99a8f2d7acd3469df7da`),
verified `manoir` at phase 9,
and read the active viewport camera through its live engine record. The viewport
is 1024×768, with horizontal FoV 1.5 radians, vertical FoV 69.8838 degrees,
near plane 0.25 and far plane 10000. At saved idle the camera was
`(-74.37785, 24.14221, 7.38520)` in source axes while Hype was
`(-74.37860, 27.48549, 4.55085)`. After a physical 500 ms Left turn, the
camera offset rotated with the actor and remained 3.3433 units behind and
2.8344 above. The recorded camera basis aims at about 1.816 units above the
actor. The source idle screenshot is
`/home/stephen/Projects/hype-harness/local/orrery-runs/run-1790230129874423799/screen-1790230162225988151.png`.

A short forward pulse showed more than a rigid orbit: camera range from Hype
was 5.118 units on release, 4.989 at +350 ms, and 4.848 at +1100 ms, then
stable. The latter pose is about 3.822 units behind and 2.982 units above Hype,
still aimed at roughly the same height. This may reflect an obstacle limit,
target-follow lag or a changed desired distance; the record alone does not
distinguish them. The time series is in the second Orrery run at
`/home/stephen/Projects/hype-harness/local/orrery-runs/run-1790230358883261554/`.
These are stopped-byte observations, and their screenshots were independent
captures, so exact frame pairing is not yet verified.

A temporary Godot calibration used the measured 4:3 projection and stationary
pose. Its visible focused BDD rerun passed both movement scenarios in one
process with complete cleanup (`TestResults/behavior-suite-1790230476038639185.json`).
At ready, its camera was `(-74.378662, 7.365318, -24.143211)` in Godot axes,
within 0.020 units of the source camera after axis conversion. The source
turn image and temporary Godot turn image framed Hype at similar size; the
Godot walk image left him roughly 30 pixels taller than the source settled-walk
image because its fixed boom stayed at the initial distance. The temporary
images are `TestResults/behavior-runs/godot-turn-release-idle-1790230522872399522.png`
and `TestResults/behavior-runs/godot-walk-release-stop-1790230525577366245.png`.
Their source and Godot frames are not synchronized, and geometry and actor
rotations differ, so landmark/pixel parity is unverified.

The user decided camera behavior should be its own development after magick.
The temporary Godot calibration and camera assertions were removed from the
movement suite. That suite continues to test actor behavior with its existing
test-owned presentation camera. The later camera slice should determine how
camera objects, selection and follow logic map to native Godot nodes, then
investigate the walk-distance change before implementing collision or easing.
