# cbit - TODO

## Steps

- [x] 1. Fix `Cargo.toml` - Change edition from "2024" to "2021"
- [x] 2. Fix `src/lua.rs` - No changes needed (already works well)
- [x] 3. Fix `src/main.rs` - Integrate Lua config loading, handle errors
- [x] 4. Fix `src/args.rs` - Major refactor: integrate Lua config, fix panics, remove duplicates, fix formatter, fix `--target`, generate `cbit.lua` on `new`
- [x] 5. `cbit new <name>` now generates `cbit.lua` inside the project dir
- [x] 6. Build & verify compilation - SUCCESS

## Soon
- [x] 1. make `src/installer.rs` - add git2 to `Cargo.toml` so i can impl git
- [ ] 2. make all installed packages go to `./lib/`
- [ ] 3. build a C compiler 
