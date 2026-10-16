# Resort Island

This level imports the supplied Sonic R course OBJ and PNG textures. The original
rip and its README are preserved in this folder; the rip is by InvisibleUp.

Sonic uses the same `GuestSonic` snapshot, `GuestPlugin`, original character
shaders, animation, camera, and pad handling as `windmill-isle guest minecraft`.
The native movement controller is disabled by `GuestMode`. Road and rock faces
use the original Havok world with adjacent-face welding data and collision layer
7. The alpha-masked flat floor uses merged fixed boxes, as the Minecraft world
does. All course collision loads before play, including complete timber and stone
bridge decks, their supports, the raised pond/waterfall meshes, and a continuous
solid ocean surface at its rendered height. Sonic can stand and run on the water.
The snapshot's original stage collision is removed. Neutral pad frames let Sonic
stop, then the course's start stripe is placed beneath his resting feet.

Run from the repository:

```powershell
.\scripts\Launch-ResortIsland.ps1
```

Use `-Build` to rebuild first. Controller support is enabled by default, alongside
keyboard input. `-KeyboardOnly` disables the gamepad backend when needed;
the existing `-Gamepad` option remains accepted.
WASD/arrows move, Space jumps, Shift/J boosts, Ctrl/K slides or stomps, Q/E are
the guest bumper inputs, R returns to the start, and Escape releases the mouse.
Gamepad bindings are identical to the Minecraft guest mode. On a PS5 controller,
the left stick moves, the right stick controls the camera, Cross jumps, Square
boosts, Circle slides or stomps, and Share returns to the start.

The default snapshot is `snapshots/82346F08_pre.bin`. Set
`UNLEASHED_SNAPSHOT` to use another compatible capture. This is required game
state, not a standalone recreation of Sonic's controller.

`level.json` controls geometry scale, the OBJ start position, sunlight, exposure,
rock/terrain brightness, sand/water tint, and ocean height/tiling. Camera motion
comes from the guest; the level applies distance, pitch and focus settings for
the reference composition, shared with the original character shaders. The sky
uses the level's cloud shader. Texture mipmaps filter the unchanged source PNGs
at distance. Resort Island interpolates camera, body and skinning poses between
the game's unchanged 60 Hz steps; the rendered model and its shadow share that
pose. Camera motion blur is disabled for this level; physics and pad steering
continue to use the original game state at 60 Hz. Display updates interpolate
between those steps and retain the window's VSync presentation.
The host reads the simulation interval from the capture, uses an f64 accumulator,
and catches up bounded elapsed display time instead of dropping it after four
steps. This preserves the original controller timing across varying render rates.

The supplied rip omits vertex colors, ring placements, and player starts. This
import supplies the start position and daylight; it is an exploration level and
does not recreate Sonic R's race rules or its missing ring placements.

For a timed capture and scripted input:

```powershell
.\scripts\Verify-ResortIsland.ps1 -Name idle -Autopilot '0:' -Seconds 9
.\scripts\Verify-ResortIsland.ps1 -Name jump-run -Autopilot '0:,3.3:j,3.7:,5:f,8:fb,10:R,10.1:' -Seconds 14
```

Captures, logs, and the settings used for each run are saved under
`artifacts/resort-island/<name>`. `course_triangle` is a headless regression that
loads the same snapshot and checks standing, jumping, and landing on imported
Havok triangles.
