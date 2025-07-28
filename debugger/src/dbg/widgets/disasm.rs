use crate::event::RequestEvent;
use crossbeam_channel::Sender;
use egui::{Color32, RichText};

pub struct DecodedInstruction {
    pub addr: u32,
    pub instr: String,
}

pub struct DisassemblyWidget {
    event_tx: Sender<RequestEvent>,
    disassembly: Vec<DecodedInstruction>,
    pc: u32,
    r15: u32,
}

// Assembly syntax highlighting colors
const OPCODE_COLOR: Color32 = Color32::from_rgb(86, 156, 214); // Blue for opcodes
const REGISTER_COLOR: Color32 = Color32::from_rgb(156, 220, 254); // Light blue for registers
const IMMEDIATE_COLOR: Color32 = Color32::from_rgb(181, 206, 168); // Green for immediate values
const ADDRESS_COLOR: Color32 = Color32::from_rgb(180, 180, 180); // Muted gray for addresses
const CONDITION_COLOR: Color32 = Color32::from_rgb(197, 134, 192); // Purple for condition codes

impl DisassemblyWidget {
    pub fn new(tx: Sender<RequestEvent>) -> DisassemblyWidget {
        let _ = tx.send(RequestEvent::UpdateDisassembly(None, 100)); // request initial disassembly

        DisassemblyWidget {
            event_tx: tx,
            disassembly: Vec::new(),
            pc: 0,
            r15: 0,
        }
    }

    pub fn update(&mut self, disassembly: Vec<DecodedInstruction>, pc: u32, r15: u32) {
        self.disassembly = disassembly;
        self.pc = pc;
        self.r15 = r15;
        let _ = self.event_tx.send(RequestEvent::UpdateDisassembly(None, 100));
    }

    fn colorize_instruction(&self, instruction: &str, base_color: Option<Color32>) -> Vec<(String, Color32)> {
        let mut tokens = Vec::new();

        // If we have a base color (PC/R15), use it for everything
        if let Some(color) = base_color {
            tokens.push((instruction.to_string(), color));
            return tokens;
        }

        // Split by whitespace to get opcode and operands
        let parts: Vec<&str> = instruction.split_whitespace().collect();
        if parts.is_empty() {
            return tokens;
        }

        // Color the opcode (with possible condition code and .s suffix)
        let opcode = parts[0];
        let mut working_opcode = opcode;
        let mut has_s_suffix = false;

        // Check for .s suffix first
        if working_opcode.ends_with(".s") {
            has_s_suffix = true;
            working_opcode = &working_opcode[..working_opcode.len() - 2];
        }

        // Check for condition codes
        if working_opcode.len() > 2 {
            let possible_condition = &working_opcode[working_opcode.len() - 2..];
            if matches!(
                possible_condition,
                "eq" | "ne" | "cs" | "cc" | "mi" | "pl" | "vs" | "vc" | "hi" | "ls" | "ge" | "lt" | "gt" | "le" | "al"
            ) {
                // Split opcode and condition
                let base_opcode = &working_opcode[..working_opcode.len() - 2];
                tokens.push((base_opcode.to_string(), OPCODE_COLOR));
                tokens.push((possible_condition.to_string(), CONDITION_COLOR));
            } else {
                tokens.push((working_opcode.to_string(), OPCODE_COLOR));
            }
        } else {
            tokens.push((working_opcode.to_string(), OPCODE_COLOR));
        }

        // Add .s suffix if present
        if has_s_suffix {
            tokens.push((".s".to_string(), CONDITION_COLOR)); // Use condition color for suffixes
        }

        // Add space if there are operands
        if parts.len() > 1 {
            tokens.push((" ".to_string(), Color32::WHITE));

            // Join the remaining parts and process as operands
            let operands_str = parts[1..].join(" ");
            let mut current_token = String::new();
            let mut chars = operands_str.chars().peekable();

            while let Some(ch) = chars.next() {
                match ch {
                    ',' => {
                        if !current_token.is_empty() {
                            tokens.push((current_token.clone(), self.get_token_color(&current_token)));
                            current_token.clear();
                        }
                        tokens.push((",".to_string(), Color32::WHITE));
                        // Skip whitespace after comma
                        while chars.peek() == Some(&' ') {
                            chars.next();
                        }
                        if chars.peek().is_some() {
                            tokens.push((" ".to_string(), Color32::WHITE));
                        }
                    }
                    ' ' => {
                        if !current_token.is_empty() {
                            tokens.push((current_token.clone(), self.get_token_color(&current_token)));
                            current_token.clear();
                        }
                        // Skip multiple spaces
                        while chars.peek() == Some(&' ') {
                            chars.next();
                        }
                        // Only add a space token if there are more non-space characters coming
                        if chars.peek().is_some() {
                            tokens.push((" ".to_string(), Color32::WHITE));
                        }
                    }
                    '[' | ']' | '{' | '}' => {
                        if !current_token.is_empty() {
                            tokens.push((current_token.clone(), self.get_token_color(&current_token)));
                            current_token.clear();
                        }
                        tokens.push((ch.to_string(), Color32::WHITE));
                    }
                    '!' => {
                        // ! can be part of addressing modes or a standalone token
                        if current_token.is_empty() {
                            tokens.push(("!".to_string(), Color32::WHITE));
                        } else {
                            current_token.push(ch);
                        }
                    }
                    _ => {
                        current_token.push(ch);
                    }
                }
            }

            // Add the last token if any
            if !current_token.is_empty() {
                tokens.push((current_token.clone(), self.get_token_color(&current_token)));
            }
        }

        tokens
    }

    fn get_token_color(&self, token: &str) -> Color32 {
        // Check for registers (including status registers)
        if token.starts_with('r')
            || token.starts_with('R')
            || matches!(token, "sp" | "lr" | "pc" | "cpsr" | "spsr")
            || token.starts_with("spsr_")
            || token.starts_with("cpsr_")
        {
            REGISTER_COLOR
        }
        // Check for immediates
        else if token.starts_with('#') || token.starts_with('+') || token.starts_with('-') {
            IMMEDIATE_COLOR
        }
        // Check for hex addresses
        else if token.starts_with("0x") {
            if token.len() > 6 {
                ADDRESS_COLOR // Long hex is likely an address
            } else {
                IMMEDIATE_COLOR
            }
        }
        // Check for hex numbers
        else if token.chars().all(|c| c.is_ascii_hexdigit()) && token.len() > 2 {
            IMMEDIATE_COLOR
        }
        // Check for instruction suffixes (.s, !)
        else if token.ends_with(".s") || token.ends_with("!") {
            // If it's an opcode with suffix, color the whole thing as opcode
            if token.len() > 2 { OPCODE_COLOR } else { Color32::WHITE }
        }
        // Check for barrel shifter (e.g., "lsl", "lsr", "asr", "ror")
        else if matches!(token, "lsl" | "lsr" | "asr" | "ror" | "rrx") {
            OPCODE_COLOR
        } else {
            Color32::WHITE
        }
    }

    pub fn render_content(&mut self, ui: &mut egui::Ui) {
        for line in self.disassembly.iter() {
            ui.horizontal(|ui| {
                let addr_label = RichText::new(format!("{:08X}", line.addr))
                    .monospace()
                    .color(ADDRESS_COLOR);

                ui.label(addr_label);

                // Create colored instruction text
                let colored_tokens = self.colorize_instruction(&line.instr, None);

                // Build a single formatted string with ANSI-like color codes
                // Since egui doesn't support inline color changes in a single RichText,
                // we'll use the job system for proper coloring
                let mut job = egui::text::LayoutJob::default();

                for (token, color) in colored_tokens {
                    job.append(
                        &token,
                        0.0,
                        egui::TextFormat {
                            font_id: egui::FontId::monospace(12.0),
                            color,
                            ..Default::default()
                        },
                    );
                }

                ui.label(job);
            });
        }
    }
}
