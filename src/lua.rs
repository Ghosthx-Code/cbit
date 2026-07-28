use mlua::{Function, Lua, Table};
use std::fs;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone)]
pub struct ProjectConfig {
    pub name: String,
    pub version: String,
    pub linker: String,
    pub std: String,
    pub format: String,
    pub include_dirs: Vec<String>,
    pub flags: Vec<String>,
    pub defines: Vec<String>,
    pub libs: Vec<String>,
}
impl Default for ProjectConfig {
    fn default() -> Self {
        Self {
            name: "unnamed_project".to_string(),
            version: "0.1.0".to_string(),
            linker: "gcc".to_string(),
            std: "c11".to_string(),
            format: "opcode".to_string(),
            include_dirs: vec![],
            flags: vec!["-Wall".to_string(), "-Wextra".to_string()],
            defines: vec![],
            libs: vec![],
        }
    }
}

pub fn load_cbit_config() -> mlua::Result<ProjectConfig> {
    let lua = Lua::new();
    let extracted_config = Arc::new(Mutex::new(ProjectConfig::default()));
    let cbit_table = lua.create_table()?;
    let config_cfg = Arc::clone(&extracted_config);
    let config_fn = lua.create_function(move |_, table: Table| {
        let mut cfg = config_cfg.lock().unwrap();
        if let Ok(name) = table.get::<_, String>("name") {
            cfg.name = name;
        }
        if let Ok(version) = table.get::<_, String>("version") {
            cfg.version = version;
        }
        if let Ok(linker) = table.get::<_, String>("linker") {
            cfg.linker = linker;
        }
        if let Ok(std_val) = table.get::<_, String>("std") {
            cfg.std = std_val;
        }
        if let Ok(format) = table.get::<_, String>("format") {
            cfg.format = format;
        }
        if let Ok(inc_dir) = table.get::<_, Vec<String>>("include_dirs") {
            cfg.include_dirs = inc_dir;
        }
        if let Ok(raw_flags) = table.get::<_, Vec<String>>("flags") {
            cfg.flags = raw_flags;
        }
        if let Ok(profile_fn) = table.get::<_, Function>("profile") {
            if let Ok(profile_flags) = profile_fn.call::<(), Vec<String>>(()) {
                cfg.flags.extend(profile_flags);
            }
        }
        if let Ok(defines) = table.get::<_, Vec<String>>("defines") {
            cfg.defines = defines;
        }
        Ok(())
    })?;
    cbit_table.set("config", config_fn)?;
    lua.globals().set("cbit", cbit_table)?;
    let script_content = fs::read_to_string("./cbit.lua").unwrap_or_else(|_| "".to_string());
    if !script_content.is_empty() {
        lua.load(&script_content).exec()?;
    }
    let final_config = extracted_config.lock().unwrap().clone();
    Ok(final_config)
}
