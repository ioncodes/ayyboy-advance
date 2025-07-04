use crate::arm7tdmi::cpu::Cpu;
use crate::arm7tdmi::decoder::Instruction;
use crate::script::proxy::Proxy;
use core::panic;
use rhai::{AST, Dynamic, Engine, Map, Scope};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use tracing::*;

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
        engine.register_fn("println", |s: &str| info!(target: "rhai", "{}", s));
        engine.register_fn("hex8", |value: i64| -> String { format!("{:02X}", value as u8) });
        engine.register_fn("hex16", |value: i64| -> String { format!("{:04X}", value as u16) });
        engine.register_fn("hex32", |value: i64| -> String { format!("{:08X}", value as u32) });
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
        engine.register_fn("disasm", |instr: i64, is_thumb: bool| -> String {
            format!(
                "{}",
                Instruction::decode(instr as u32, is_thumb).unwrap_or(Instruction::nop())
            )
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
        engine.register_fn("is_thumb", |proxy: &mut Proxy| -> bool { proxy.is_thumb() });

        Self {
            engine,
            breakpoint_handlers: HashMap::new(),
            script: None,
            loaded: false,
        }
    }

    pub fn load_script(&mut self, script_path: &Path) {
        if !script_path.exists() {
            panic!("Script file {} does not exist", script_path.display());
        }

        let script_content = match fs::read_to_string(&script_path) {
            Ok(content) => content,
            Err(e) => {
                panic!("Failed to read script file {}: {}", script_path.display(), e);
            }
        };

        let ast = match self.engine.compile(&script_content) {
            Ok(ast) => ast,
            Err(e) => {
                panic!("Failed to compile script {}: {}", script_path.display(), e);
            }
        };

        // Cache the AST for later use
        self.script = Some(ast.clone());

        // Call the setup functions and grab the breakpoints
        let mut scope = Scope::new();
        match self.engine.call_fn::<Dynamic>(&mut scope, &ast, "setup", ()) {
            Ok(result) => {
                if self.parse_breakpoints(result) {
                    info!(target: "rhai",
                        "Loaded {} breakpoint(s) from script {}",
                        self.breakpoint_handlers.len(),
                        script_path.display()
                    );
                } else {
                    panic!("Failed to parse breakpoints from script {}", script_path.display());
                }
            }
            Err(e) => {
                panic!("Failed to execute setup() in script {}: {}", script_path.display(), e);
            }
        }
    }

    pub fn handle_breakpoint(&mut self, address: u32, instr_addr: u32, cpu: &mut Cpu) {
        if !self.loaded || !self.breakpoint_handlers.contains_key(&address) {
            return;
        }

        let handler_name = match self.breakpoint_handlers.get(&address) {
            Some(name) => name,
            None => return,
        };

        if let Some(ast) = &self.script {
            let mut scope = Scope::new();
            scope.push("emu", Proxy::new(cpu));
            scope.push("addr", instr_addr as i64);

            // call the handler
            match self.engine.call_fn::<()>(&mut scope, &ast, handler_name, ()) {
                Ok(_) => {
                    debug!(target: "rhai",
                        "Executed script handler '{}' for breakpoint at 0x{:08X}",
                        handler_name, address
                    );
                }
                Err(e) => panic!(
                    "Failed to execute handler '{}' for breakpoint at 0x{:08X}: {}",
                    handler_name, address, e
                ),
            }
        }
    }

    fn parse_breakpoints(&mut self, result: Dynamic) -> bool {
        if let Some(map) = result.try_cast::<Map>() {
            for (addr_key, handler_value) in map.iter() {
                let addr_str = addr_key.to_string();

                if !addr_str.starts_with("0x") {
                    error!(target: "rhai", "Invalid breakpoint address format: {}", addr_str);
                    continue;
                }

                let addr_value = match u32::from_str_radix(&addr_str[2..], 16) {
                    Ok(value) => value,
                    Err(_) => {
                        error!(target: "rhai", "Can't parse breakpoint address: {}", addr_str);
                        continue;
                    }
                };

                // Extract handler function name
                if let Some(handler_name) = handler_value.clone().try_cast::<String>() {
                    self.breakpoint_handlers.insert(addr_value, handler_name.clone());
                    debug!(target: "rhai", "Added breakpoint at {} with handler '{}'", addr_str, handler_name);
                } else {
                    error!(target: "rhai", "Handler for address {} is not a function name string", addr_str);
                }
            }

            self.loaded = true;
            true
        } else {
            error!(target: "rhai", "setup() did not return a map");
            false
        }
    }
}
