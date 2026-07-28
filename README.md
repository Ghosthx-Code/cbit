# CBIT README

## Commands
```bash
cbit new <name> # makes a new project
cbit build # in the project dir, you can use --target <arch> after to compile to a arch
cbit fmt # formatter
cbit run # runs the project
cbit clean # removes the .build dir
cbit -h # displays help menu
```

## To Install
```bash
git clone https://github.com/Ghosthx-Code/cbit
cd cbit
cargo build --release
mv target/release/cbit .
sudo mv cbit /usr/local/bin/cbit
cbit --help # make sure it works
```

## Config
wen you make a new project there will be a `cbit.lua` file 
```lua
cbit.config({
  name = "tmp_c", -- name 
  version = "0.1.0", -- version of exe
  linker = "gcc", -- linker 
  std = "c11", -- the std version
  format = "opcode", -- format, opcode/bitcode
  include_dirs = { "lib" }, -- this is for the libs, i will impl install packages soon
  flags = { "-Wall", "-Wextra" }, -- linkers flags
  defines = { "NDEBUG" }, -- define
  libs = { "./lib" }, -- lib
  profile = function() -- profile stuff
    local profile = os.getenv("CBIT_PROFILE") or "debug"
    if profile == "release" then
      return { "-O3", "-march=native" }
    elseif profile == "debug" then
      return { "-O0", "-g" }
    else
      return { "-O2" }
    end
  end,
})

```
