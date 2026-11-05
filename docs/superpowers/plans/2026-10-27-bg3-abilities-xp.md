# BG3: edit ability scores and XP

**Goal:** the BG3 plugin edits each player character's ability scores and
total XP, and the game loads the edited save without a "tampered" warning.

## Findings (verified on a 4.1.1.7631656 save, 2026-10-27)

Character state lives in the `NewAge` attribute (a `ScratchBuffer`) of the
`NewAge` region in `Globals.lsf`: an ECS dump with the magic `LSMF`. Format
notes from [bg3-newage](https://github.com/NodeJSmith/bg3-newage) and
[bg3-party-tool](https://github.com/nirnx/bg3-party-tool) (both MIT), checked
against the real blob. We write our own implementation.

- **Header, 48 bytes:** `"LSMF"`, `u32 version` (`0x08000101`), `u64 hash`,
  `u64 blockOffset`, `u64 blockSize`, `u32 nameBlockSize`, `u16 componentCount`,
  `u16`, `u64`. Offsets below are relative to the end of the header.
- **Checksum:** `hash` is `h1` of MurmurHash3 x64_128 over every byte after the
  header, seed `0xf0015eed5eedf001`. (bg3-party-tool says `h1 + h2` over a
  narrower range; that does not match.)
- **Names:** a packed string block at `blockOffset`, `nameBlockSize` bytes.
- **Component table:** right after the names, 48 bytes per component:
  `u64 nameOffset`, `u32 nameSize`, `u32` (always 101 here), `u64 hash`,
  `u32 elementSize`, `u32 version`, `u64 elementCount`, `u64 dataOffset`.
- **`core.v0.Level`** (component 0): `u64 entitiesStart, entitiesEnd,
  ownerMapsStart, ownerMapsEnd`. Entities are 16-byte GUIDs. Each owner map is
  `u64 indicesStart, indicesEnd, componentIndex, count`, pointing at `u32`
  entity indices: element `k` belongs to entity `indices[k]`. An empty map
  stores `u64::MAX`.
- **`game.v0.PlayerComponent`** (1 byte): its owners are the player characters.
- **`game.stats.v3.StatsComponent`** (36 bytes): 8 unknown bytes, `i32`
  STR DEX CON INT WIS CHA, `i32` proficiency bonus.
- **`game.experience.v0.ExperienceComponent`** (12 bytes): `i32` total XP
  twice, 4 unknown bytes. Matches `SaveInfo.json`, which is how members get names.
- **`CharacterCreationStatsComponent`** (88 bytes) points into side tables;
  base abilities may live there. Not decoded yet.
- **Package hash:** the LSPK v18 header MD5 is over the uncompressed contents
  of all files sorted by **lower-cased** path, each byte + 1. Verified against
  the real save. `lslib` currently hashes in package order and `to_builder`
  turns hashing off.

**Open question:** does the game keep an edited `StatsComponent`, or rebuild
it from the character-creation and level-up data on load? Task 3 answers it
in game before any UI is built.

## Tasks

1. **lslib: BG3 archive hash.** v15+ hashes files sorted by lower-cased path;
   `to_builder` keeps hashing on. Test the ordering; the real-save test checks
   that an unchanged repack reproduces the original MD5.
2. **lslib: `newage` module.** Parse the header, names, component table,
   entities and owner maps. Look up a component by name and get its elements
   with their owning entity. Overwrite an element in place (same size). Write
   back with a fresh checksum. Tests: a synthetic blob built in the test;
   real-save round trip is byte-identical and the stored checksum verifies.
3. **Experiment (user, in game).** An example writes a copy of the test save
   with one player's STR +2 and XP +100. Load it in the game and report:
   loads without warning? STR changed? XP changed?
4. **bg3 plugin: read players.** Players from `PlayerComponent`; abilities
   and XP; names by XP against `SaveInfo.json`, else `Character N`.
5. **bg3 plugin: editor.** A character pick-list, ability number inputs
   (1–30) and XP. `serialize` patches the blob, re-embeds it in `Globals.lsf`
   and repacks with the BG3 hash. Keeps the overview sections.

## Outcome (2026-10-27)

All tasks done. The in-game experiments showed:

- **XP** edits in `ExperienceComponent` stick.
- **`StatsComponent` is derived:** the game rebuilds it on load, so editing it
  alone has no effect.
- **Base scores** come from `CharacterCreationStatsComponent`. Its 88-byte
  element holds, at byte 56, a `u64` name offset and a `u32` name length (the
  character's name, e.g. `Gale`), and at byte 72 a `u64` start/end of an `i32`
  array indexed None, STR..CHA. The array holds the **points spent over 8** at
  character creation; racial and other bonuses are added on top.
- **Editing the points** changes the in-game scores, both above the point-buy
  cap (+10 → STR 18) and below it (−3 → DEX 5).

The editor therefore changes a score by moving those points by the same
amount, and updates `StatsComponent` to match. Repacking keeps the stored bytes
of unchanged files (`InputFile::stored`), so only `Globals.lsf` is
recompressed. Serialize takes about 0.8 s natively in release on a 16 MB save.

Not covered yet: `SaveInfo.json` (load-screen XP and party) is not updated, and
companions in camp have no `ExperienceComponent`, so their XP is not editable.
