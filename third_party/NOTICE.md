# Third-party code

- `minecraftoss/` (core, generator, world, player, entities): MinecraftOSS engine crates
  as vendored by 2010-rust-rewrite-mashup (third_party/minecraftoss, from MinecraftOSS
  commit 4013a68). The copy carries no license file; confirm upstream's license before
  redistributing.
- `minecraft_terrain/`: from 2010-rust-rewrite-mashup (IW4L), crates/minecraft_terrain at
  mashup commit f608f85, Apache License 2.0 (see the mashup's LICENSE/NOTICE). It is
  MinecraftOSS's engine/viewer (commit 9e4108d) without its GPU binding code.

No Minecraft code, JARs or assets are included: the game downloads Minecraft 26.3's own
files from Mojang's servers on first run (crates/unleashed-minecraft/src/setup.rs).


Export attribution update: LICENSE-mashup and NOTICE-mashup are unmodified copies
from chasmlol/2010-rust-rewrite-mashup at f608f85. Its NOTICE identifies the in-tree
MinecraftOSS code as its own author's project; these retain the upstream terms.
The local integration adapts workspace/path dependencies and uses the terrain
viewer implementation without its original GPU bindings. This source release
retains the existing local adaptations and does not imply a new project-wide license.
