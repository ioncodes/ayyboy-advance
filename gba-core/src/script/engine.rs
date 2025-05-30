use crate::arm7tdmi::cpu::Cpu;
use crate::script::proxy::Proxy;
use log::*;
use rhai::{Dynamic, Engine, Map, Scope, AST};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

pub struct ScriptEngine {
    engine: Engine,
    breakpoint_handlers: HashMap<u32, String>,
    script: Option<AST>,
    loaded: bool,
}

impl ScriptEngine {
    pub fn new() -> Self {
        let mut engine = Engine::new();

        // Helper functions
        engine.register_fn("println", |s: &str| info!("[RHAI] {}", s));
        engine.register_fn("hex8", |value: i64| -> String { format!("{:02x}", value as u8) });
        engine.register_fn("hex16", |value: i64| -> String { format!("{:04x}", value as u16) });
        engine.register_fn("hex32", |value: i64| -> String { format!("{:08x}", value as u32) });
        engine.register_fn("bin8", |value: i64| -> String { format!("{:08b}", value as u8) });
        engine.register_fn("bin16", |value: i64| -> String { format!("{:016b}", value as u16) });
        engine.register_fn("bin32", |value: i64| -> String { format!("{:032b}", value as u32) });
        engine.register_fn("padleft", |s: &str, token: &str, len: i64| -> String {
            let mut padded = s.to_string();
            while padded.len() < len as usize {
                padded.insert(0, token.chars().next().unwrap());
            }
            padded
        });
        engine.register_fn("padright", |s: &str, token: &str, len: i64| -> String {
            let mut padded = s.to_string();
            while padded.len() < len as usize {
                padded.push(token.chars().next().unwrap());
            }
            padded
        });

        // proxy struct
        engine.register_type::<Proxy>();
        engine.register_fn("read_u8", |proxy: &mut Proxy, address: i64| -> i64 {
            proxy.read_u8(address) as i64
        });
        engine.register_fn("read_u16", |proxy: &mut Proxy, address: i64| -> i64 {
            proxy.read_u16(address) as i64
        });
        engine.register_fn("read_u32", |proxy: &mut Proxy, address: i64| -> i64 {
            proxy.read_u32(address) as i64
        });
        engine.register_fn("write_u8", |proxy: &mut Proxy, address: i64, value: i64| {
            proxy.write_u8(address, value);
        });
        engine.register_fn("write_u16", |proxy: &mut Proxy, address: i64, value: i64| {
            proxy.write_u16(address, value);
        });
        engine.register_fn("write_u32", |proxy: &mut Proxy, address: i64, value: i64| {
            proxy.write_u32(address, value);
        });
        engine.register_fn("read_register", |proxy: &mut Proxy, reg: &str| -> i64 {
            proxy.read_register(reg) as i64
        });
        engine.register_fn("write_register", |proxy: &mut Proxy, reg: &str, value: i64| {
            proxy.write_register(reg, value as u32);
        });
        engine.register_fn("read_cpsr", |proxy: &mut Proxy| -> i64 { proxy.read_cpsr() as i64 });

        Self {
            engine,
            breakpoint_handlers: HashMap::new(),
            script: None,
            loaded: false,
        }
    }

    pub fn load_script(&mut self, script_path: &Path) -> bool {
        if !script_path.exists() {
            error!("Script file not found: {}", script_path.display());
            return false;
        }

        let script_content = match fs::read_to_string(&script_path) {
            Ok(content) => content,
            Err(e) => {
                error!("Failed to read script file {}: {}", script_path.display(), e);
                return false;
            }
        };

        // Compile it
        let ast = match self.engine.compile(&script_content) {
            Ok(ast) => ast,
            Err(e) => {
                error!("Failed to compile script: {}", e);
                return false;
            }
        };

        // Cache the AST for later use
        self.script = Some(ast.clone());

        // Call the setup functions and grab the breakpoints
        let mut scope = Scope::new();
        match self.engine.call_fn::<Dynamic>(&mut scope, &ast, "setup", ()) {
            Ok(result) => {
                if self.parse_breakpoints(result) {
                    info!(
                        "Loaded {} breakpoint(s) from script {}",
                        self.breakpoint_handlers.len(),
                        script_path.display()
                    );
                    true
                } else {
                    error!("Failed to parse breakpoints from script.");
                    false
                }
            }
            Err(e) => {
                error!("Failed to get breakpoints from script: {}", e);
                false
            }
        }
    }

    pub fn handle_breakpoint(&mut self, address: u32, cpu: &mut Cpu) -> bool {
        if !self.loaded || !self.breakpoint_handlers.contains_key(&address) {
            return false;
        }

        let handler_name = match self.breakpoint_handlers.get(&address) {
            Some(name) => name,
            None => return false,
        };

        if let Some(ast) = &self.script {
            let mut scope = Scope::new();
            scope.push("emu", Proxy::new(cpu));

            // call the handler
            match self.engine.call_fn::<()>(&mut scope, &ast, handler_name, ()) {
                Ok(_) => {
                    debug!(
                        "Executed script handler '{}' for breakpoint at 0x{:08x}",
                        handler_name, address
                    );
                    return true;
                }
                Err(e) => error!("Error executing script handler '{}': {}", handler_name, e),
            }
        }

        false
    }

    fn parse_breakpoints(&mut self, result: Dynamic) -> bool {
        if let Some(map) = result.try_cast::<Map>() {
            for (addr_key, handler_value) in map.iter() {
                let addr_str = addr_key.to_string();

                if !addr_str.starts_with("0x") {
                    error!("Invalid breakpoint address format: {}", addr_str);
                    continue;
                }

                let addr_value = match u32::from_str_radix(&addr_str[2..], 16) {
                    Ok(value) => value,
                    Err(_) => {
                        error!("Can't parse breakpoint address: {}", addr_str);
                        continue;
                    }
                };

                // Extract handler function name
                if let Some(handler_name) = handler_value.clone().try_cast::<String>() {
                    self.breakpoint_handlers.insert(addr_value, handler_name.clone());
                    debug!("Added breakpoint at {} with handler '{}'", addr_str, handler_name);
                } else {
                    error!("Handler for address {} is not a function name string", addr_str);
                }
            }

            self.loaded = true;
            true
        } else {
            error!("setup() did not return a map");
            false
        }
    }
}
