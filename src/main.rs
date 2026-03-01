// use rand::Rng;

extern crate rand;

// extern crate ggez;
use std::fs;
use std::fs::File;
use std::io::prelude::*;
use std::path::{Path, PathBuf};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use eframe::egui;
use egui::{Color32, RichText, ScrollArea};
use rodio::source::SineWave;
use rodio::{OutputStream, OutputStreamBuilder, Sink, Source};

pub fn print_binary(bytes: &Vec<u8>) {
    for x in bytes.iter() {
        print!("{:x}", x)
    }
    println!("");
}

pub fn print_display(bytes: &Vec<bool>) {
    for _ in 0..64 {
        print!("-");
    }
    print!("\n");

    for (i, x) in bytes.iter().enumerate() {
        if *x {
            print!("0");
        } else {
            print!(" ");
        }
        if i % 64 == 0 && i > 0 {
            println!("");
        }
    }
    print!("\n");
    for _ in 0..64 {
        print!("-");
    }
    println!("");
}

pub fn read_game(path: &str) -> Vec<u8> {
    let path = Path::new(path);
    let display = path.display();

    let mut file = match File::open(path) {
        Err(why) => panic!("Couldn't open file {}: {}", display, why),
        Ok(file) => file,
    };

    let mut game = Vec::new();
    match file.read_to_end(&mut game) {
        Err(why) => panic!("Couldn't read file {}: {}", display, why),
        Ok(_) => (),
    };

    // print_binary(&game);

    game
}

#[derive(Clone)]
pub struct Keypad {
    pub keys: [bool; 16],
}

impl Keypad {
    pub fn new() -> Keypad {
        let keypad = Keypad { keys: [false; 16] };

        keypad
    }
}

#[derive(Clone)]
pub struct Display {
    pub memory: [bool; 2048],
}

impl Display {
    pub fn new() -> Display {
        let display = Display {
            memory: [false; 2048],
        };

        display
    }

    fn clear(&mut self) {
        self.memory = [false; 2048]
    }
}

static FONTSET: [u8; 80] = [
    0xF0, 0x90, 0x90, 0x90, 0xF0, // 0
    0x20, 0x60, 0x20, 0x20, 0x70, // 1
    0xF0, 0x10, 0xF0, 0x80, 0xF0, // 2
    0xF0, 0x10, 0xF0, 0x10, 0xF0, // 3
    0x90, 0x90, 0xF0, 0x10, 0x10, // 4
    0xF0, 0x80, 0xF0, 0x10, 0xF0, // 5
    0xF0, 0x80, 0xF0, 0x90, 0xF0, // 6
    0xF0, 0x10, 0x20, 0x40, 0x40, // 7
    0xF0, 0x90, 0xF0, 0x90, 0xF0, // 8
    0xF0, 0x90, 0xF0, 0x10, 0xF0, // 9
    0xF0, 0x90, 0xF0, 0x90, 0x90, // A
    0xE0, 0x90, 0xE0, 0x90, 0xE0, // B
    0xF0, 0x80, 0x80, 0x80, 0xF0, // C
    0xE0, 0x90, 0x90, 0x90, 0xE0, // D
    0xF0, 0x80, 0xF0, 0x80, 0xF0, // E
    0xF0, 0x80, 0xF0, 0x80, 0x80, // F
];

#[derive(Clone)]
pub struct Cpu {
    // index register
    pub i: u16,
    // program counter
    pub pc: u16,
    // memory
    pub memory: [u8; 4096],
    // registers
    pub v: [u8; 16],
    // peripherals
    pub keypad: Keypad,
    pub display: Display,
    // stack
    pub stack: [u16; 16],
    // stack pointer
    pub sp: u8,
    // delay timer
    pub dt: u8,
    // sound timer
    pub st: u8,
    // Internal cycle divider for 60 Hz timers.
    pub tick_divider_counter: u32,
    pub key_detection_index: usize,
    pub super_chip: bool,
    pub display_height: u8,
    pub display_width: u8,
}

fn read_word(memory: &[u8; 4096], counter: u16) -> u16 {
    (memory[counter as usize] as u16) << 8 | (memory[(counter + 1) as usize] as u16)
}

impl Cpu {
    pub fn new() -> Cpu {
        let mut cpu = Cpu {
            memory: [0; 4096],
            v: [0; 16],
            i: 0x200,
            pc: 0x200,
            stack: [0; 16],
            sp: 0,
            dt: 0,
            st: 0,
            tick_divider_counter: 0,
            keypad: Keypad::new(),
            display: Display::new(),
            key_detection_index: 0,
            super_chip: false,
            display_height: 32,
            display_width: 64,
        };

        cpu.memory[..FONTSET.len()].clone_from_slice(&FONTSET);

        // print_binary(&cpu.memory.to_vec());

        cpu
    }

    pub fn load_game(&mut self, game: &[u8]) {
        let start = 512;
        let max_len = self.memory.len().saturating_sub(start);
        let game_len = game.len().min(max_len);
        let end = start + game_len;
        self.memory[start..end].clone_from_slice(&game[..game_len]);

        // print_binary(&self.memory.to_vec());
    }

    pub fn tick(&mut self) {
        if self.dt > 0 {
            self.dt -= 1;
        }

        if self.st > 0 {
            // Beep the beeper
            if self.st == 1 {}

            self.st -= 1;
        }
    }

    pub fn execute_cycle(&mut self) {
        if (self.pc as usize) > self.memory.len().saturating_sub(1) {
            return;
        }
        // println!("{}",  self.pc);
        let opcode: u16 = read_word(&self.memory, self.pc);
        self.pc = self.pc.saturating_add(2);
        self.tick_divider_counter = self.tick_divider_counter.saturating_add(1);
        if self.tick_divider_counter >= TICK_DIVISOR {
            self.tick_divider_counter = 0;
            self.tick();
        }
        self.process_opcode(opcode);
    }

    pub fn current_opcode(&self) -> Option<u16> {
        if (self.pc as usize) >= self.memory.len().saturating_sub(1) {
            return None;
        }
        Some(read_word(&self.memory, self.pc))
    }

    pub fn split_u4(&mut self, opcode: u16) -> Vec<u8> {
        let u4_0: u8 = ((opcode & 0xf000) >> 12) as u8;
        let u4_1: u8 = ((opcode & 0x0f00) >> 8) as u8;
        let u4_2: u8 = ((opcode & 0x00f0) >> 4) as u8;
        let u4_3: u8 = (opcode & 0x000f) as u8;

        vec![u4_0, u4_1, u4_2, u4_3]
    }

    fn process_opcode(&mut self, opcode: u16) {
        // println!("processing opcode, {:x}", opcode);

        let separate_bytes = self.split_u4(opcode);

        // print_binary(&separate_bytes);

        let kk = opcode & 0x00ff;
        let nnn = opcode & 0x0fff;
        let n = opcode & 0x000f;

        match separate_bytes[..] {
            [0, 0, 0xE, 0] => self.cls(),
            [0, 0, 0xE, 0xE] => self.ret(),
            [0, _, _, _] => self.sys(),
            [1, _, _, _] => self.jp(nnn),
            [2, _, _, _] => self.call(nnn),
            [3, x, _, _] => self.se(x, kk),
            [4, x, _, _] => self.sen(x, kk),
            [5, x, y, 0] => self.sexy(x, y),
            [6, x, _, _] => self.ldxkk(x, kk),
            [7, x, _, _] => self.addxkk(x, kk),
            [8, x, y, 0] => self.ldxy(x, y),
            [8, x, y, 1] => self.or(x, y),
            [8, x, y, 2] => self.and(x, y),
            [8, x, y, 3] => self.xor(x, y),
            [8, x, y, 4] => self.add(x, y),
            [8, x, y, 5] => self.sub(x, y),
            [8, x, y, 6] => self.shr(x, y),
            [8, x, y, 7] => self.subn(x, y),
            [8, x, y, 0xE] => self.shl(x, y),
            [9, x, y, 0] => self.sne(x, y),
            [0xA, _, _, _] => self.ldi(nnn),
            [0xB, _, _, _] => self.jpv0(nnn),
            [0xC, x, _, _] => self.rnd(x, kk),
            [0xD, x, y, _] => self.drw(x, y, n as u8),
            [0xE, x, 0x9, 0xE] => self.skp(x),
            [0xE, x, 0xA, 0x1] => self.sknp(x),
            [0xF, x, 0x0, 0x7] => self.ld_v_dt(x),
            [0xF, x, 0x0, 0xA] => self.ld_k(x),
            [0xF, x, 0x1, 0x5] => self.ld_dt_v(x),
            [0xF, x, 0x1, 0x8] => self.ld_st(x),
            [0xF, x, 0x1, 0xE] => self.add_i(x),
            [0xF, x, 0x2, 0x9] => self.ld_f(x),
            [0xF, x, 0x3, 0x3] => self.ld_b(x),
            [0xF, x, 0x5, 0x5] => self.ld_i_v(x),
            [0xF, x, 0x6, 0x5] => self.ld_v_i(x),
            _ => println!("Unimplemented opcode: {:x}", opcode),
        }
    }

    //
    // Opcodes
    //

    // 0nnn - SYS addr
    // Jump to a machine code routine at nnn.
    // This instruction is only used on the old computers on which Chip-8 was originally
    // implemented. It is ignored by modern interpreters.
    fn sys(&mut self) {}

    // 00E0 - CLS
    // Clear the display.
    fn cls(&mut self) {
        self.display.clear();
    }

    // 00EE - RET
    // Return from a subroutine.
    // The interpreter sets the program counter to the address at the top of the stack, then
    // subtracts 1 from the stack pointer.
    fn ret(&mut self) {
        self.pc = *self.stack.get(self.sp as usize).unwrap();
        self.sp = self.sp.saturating_sub(1);
    }

    // 1nnn - JP addr
    // Jump to location nnn.
    // The interpreter sets the program counter to nnn.
    fn jp(&mut self, nnn: u16) {
        self.pc = nnn;
    }

    // 2nnn - CALL addr
    // Call subroutine at nnn.
    // The interpreter increments the stack pointer, then puts the current PC on the top of the
    // stack. The PC is then set to nnn.
    fn call(&mut self, nnn: u16) {
        self.sp = self.sp.wrapping_add(1);
        self.stack[self.sp as usize] = self.pc;
        self.pc = nnn;
    }

    // 3xkk - SE Vx, byte
    // Skip next instruction if Vx = kk.
    // The interpreter compares register Vx to kk, and if they are equal, increments the program
    // counter by 2.
    fn se(&mut self, x: u8, kk: u16) {
        if self.v[x as usize] as u16 == kk {
            self.pc += 2;
        } else {
        }
    }

    // 4xkk - SNE Vx, byte
    // Skip next instruction if Vx != kk.
    // The interpreter compares register Vx to kk, and if they are not equal, increments the
    // program counter by 2.
    fn sen(&mut self, x: u8, kk: u16) {
        if self.v[x as usize] as u16 != kk {
            self.pc += 2;
        } else {
        }
    }

    // 5xy0 - SE Vx, Vy
    // Skip next instruction if Vx = Vy.
    // The interpreter compares register Vx to register Vy, and if they are equal, increments the
    // program counter by 2.
    fn sexy(&mut self, x: u8, y: u8) {
        if self.v[x as usize] == self.v[y as usize] {
            self.pc += 2;
        } else {
        }
    }

    // 6xkk - LD Vx, byte
    // Set Vx = kk.
    // The interpreter puts the value kk into register Vx.
    fn ldxkk(&mut self, x: u8, kk: u16) {
        self.v[x as usize] = kk as u8;
    }

    // 7xkk - ADD Vx, byte
    // Set Vx = Vx + kk.
    // Adds the value kk to the value of register Vx, then stores the result in Vx.
    fn addxkk(&mut self, x: u8, kk: u16) {
        let mut vx = self.v[x as usize];
        vx = vx.wrapping_add(kk as u8);
        self.v[x as usize] = vx;
    }

    // 8xy0 - LD Vx, Vy
    // Set Vx = Vy.
    // Stores the value of register Vy in register Vx.
    fn ldxy(&mut self, x: u8, y: u8) {
        self.v[x as usize] = self.v[y as usize];
    }

    // 8xy1 - OR Vx, Vy
    // Set Vx = Vx OR Vy.
    // Performs a bitwise OR on the values of Vx and Vy, then stores the result in Vx.
    fn or(&mut self, x: u8, y: u8) {
        let res: u8 = self.v[x as usize] | self.v[y as usize];
        self.v[x as usize] = res;
        self.v[0xf] = 0;
    }

    // 8xy2 - AND Vx, Vy
    // Set Vx = Vx AND Vy.
    // Performs a bitwise AND on the values of Vx and Vy, then stores the result in Vx.
    fn and(&mut self, x: u8, y: u8) {
        let res: u8 = self.v[x as usize] & self.v[y as usize];
        self.v[x as usize] = res;
        self.v[0xf] = 0;
    }

    // 8xy3 - XOR Vx, Vy
    // Set Vx = Vx XOR Vy.
    // Performs a bitwise exclusive OR on the values of Vx and Vy, then stores the result in Vx.
    fn xor(&mut self, x: u8, y: u8) {
        let res: u8 = self.v[x as usize] ^ self.v[y as usize];
        self.v[x as usize] = res;
        self.v[0xf] = 0;
    }

    // 8xy4 - ADD Vx, Vy
    // Set Vx = Vx + Vy, set VF = carry.
    // The values of Vx and Vy are added together. If the result is greater than 8 bits (i.e.,
    // > 255,) VF is set to 1, otherwise 0. Only the lowest 8 bits of the result are kept, and
    // stored in Vx.
    fn add(&mut self, x: u8, y: u8) {
        let vx = self.v[x as usize];
        let vy = self.v[y as usize];
        let (new_vx, carry) = vx.overflowing_add(vy);
        let vf: u8 = if carry { 1 } else { 0 };

        self.v[x as usize] = new_vx;
        self.v[0xf] = vf;
    }

    // 8xy5 - SUB Vx, Vy
    // Set Vx = Vx - Vy, set VF = NOT borrow.
    // If Vx > Vy, then VF is set to 1, otherwise 0. Then Vy is subtracted from Vx, and the results
    // stored in Vx.
    fn sub(&mut self, x: u8, y: u8) {
        let vx = self.v[x as usize];
        let vy = self.v[y as usize];
        let (res, borrow) = vx.overflowing_sub(vy);
        let carry: u8 = if borrow { 0 } else { 1 };

        self.v[x as usize] = res;
        self.v[0xf] = carry;
    }

    // 8xy6 - SHR Vx {, Vy}
    // Set Vx = Vx SHR 1.
    // If the least-significant bit of Vx is 1, then VF is set to 1, otherwise 0.  Then Vx is
    // divided by 2.
    fn shr(&mut self, x: u8, y: u8) {
        // NOTE: ignore y if CHIP-48 and SUPER-CHIP
        let vx;
        if self.super_chip {
            vx = self.v[x as usize];
        } else {
            vx = self.v[y as usize];
        }
        let carry: u8 = vx & 0x1;
        let res = vx >> 1;

        self.v[x as usize] = res;
        self.v[0xf] = carry;
    }

    // 8xy7 - SUBN Vx, Vy
    // Set Vx = Vy - Vx, set VF = NOT borrow.
    // If Vy > Vx, then VF is set to 1, otherwise 0. Then Vx is subtracted from Vy, and the results
    // stored in Vx.
    fn subn(&mut self, x: u8, y: u8) {
        let vx = self.v[x as usize];
        let vy = self.v[y as usize];
        let (res, borrow) = vy.overflowing_sub(vx);
        let carry: u8 = if borrow { 0 } else { 1 };

        self.v[x as usize] = res;
        self.v[0xf] = carry;
    }

    // 8xyE - SHL Vx {, Vy}
    // Set Vx = Vx SHL 1.
    // If the most-significant bit of Vx is 1, then VF is set to 1, otherwise to
    // 0. Then Vx is multiplied by 2.
    fn shl(&mut self, x: u8, y: u8) {
        // NOTE: ignore y if CHIP-48 and SUPER-CHIP
        let vx;
        if self.super_chip {
            vx = self.v[x as usize];
        } else {
            vx = self.v[y as usize];
        }

        let carry: u8 = (vx >> 7) & 0x1;
        let res = vx << 1;

        self.v[x as usize] = res;
        self.v[0xf] = carry;
    }

    // 9xy0 - SNE Vx, Vy
    // Skip next instruction if Vx != Vy.
    // The values of Vx and Vy are compared, and if they are not equal, the program counter is
    // increased by 2.
    fn sne(&mut self, x: u8, y: u8) {
        let vx = self.v[x as usize];
        let vy = self.v[y as usize];

        if vx != vy {
            self.pc += 2;
        } else {
        }
    }

    // Annn - LD I, addr
    // Set I = nnn.
    // The value of register I is set to nnn.
    fn ldi(&mut self, nnn: u16) {
        self.i = nnn;
    }

    // Bnnn - JP V0, addr
    // Jump to location nnn + V0.
    // The program counter is set to nnn plus the value of V0.
    fn jpv0(&mut self, nnn: u16) {
        self.pc = nnn + (self.v[0] as u16);
    }

    // Cxkk - RND Vx, byte
    // Set Vx = random byte AND kk.
    // The interpreter generates a random number from 0 to 255, which is then ANDed with the value
    // kk. The results are stored in Vx. See instruction 8xy2 for more information on AND.
    fn rnd(&mut self, x: u8, kk: u16) {
        let rn = rand::random::<u8>();
        self.v[x as usize] = ((rn as u16) & kk) as u8;
    }

    // Dxyn - DRW Vx, Vy, nibble
    // Display n-byte sprite starting at memory location I at (Vx, Vy), set VF = collision.

    // The interpreter reads n bytes from memory, starting at the address stored in I.
    // These bytes are then displayed as sprites on screen at coordinates (Vx, Vy).
    // Sprites are XORed onto the existing screen. If this causes any pixels to be erased,
    // VF is set to 1, otherwise it is set to 0. If the sprite is positioned so part of it
    // is outside the coordinates of the display, it wraps around to the opposite side of the screen.
    // See instruction 8xy3 for more information on XOR
    fn drw(&mut self, x: u8, y: u8, n: u8) {
        // running at timer clock speed is apparently too slow.
        // running at about 2x the timer clock speed seems to be ok
        if self.tick_divider_counter < (TICK_DIVISOR / 2) {
            // Start with no collision
            self.v[0xf] = 0;

            let x_pos = self.v[x as usize] as u16;
            let y_pos = self.v[y as usize] as u16;

            // For each row of the sprite
            for row in 0..n {
                // Get sprite byte from memory at I + row
                let sprite_byte = self.memory[(self.i + row as u16) as usize];
                let screen_y = (y_pos + row as u16) % self.display_height as u16;

                // For each pixel in the row (8 pixels per byte)
                for col in 0..8u16 {
                    // Calculate screen position with wraparound
                    let screen_x = (x_pos + col) % self.display_width as u16;
                    let idx = (screen_y * self.display_width as u16 + screen_x) as usize;

                    // Check if this pixel is set in the sprite
                    if (sprite_byte & (0b1000_0000 >> col)) > 0 {
                        // XOR pixel onto screen, check for collision
                        if self.display.memory[idx] {
                            self.v[0xf] = 1;
                        }

                        self.display.memory[idx] ^= true;
                    }

                    if screen_x == self.display_width as u16 - 1 {
                        break;
                    }
                }

                if screen_y == self.display_height as u16 - 1 {
                    break;
                }
            }
        } else {
            // Loop until we get the tick (blanking interval)
            self.pc -= 2;
        }
    }

    // Ex9E - SKP Vx
    // Skip next instruction if key with the value of Vx is pressed.
    //
    // Checks the keyboard, and if the key corresponding to the value
    // of Vx is currently in the down position, PC is increased by 2.
    fn skp(&mut self, x: u8) {
        let key = self.v[x as usize] as usize;
        if self.keypad.keys[key] {
            self.pc += 2;
        } else {
        }
    }

    // ExA1 - SKNP Vx
    // Skip next instruction if key with the value of Vx is not pressed.
    //
    // Checks the keyboard, and if the key corresponding to the value of Vx
    // is currently in the up position, PC is increased by 2.
    fn sknp(&mut self, x: u8) {
        let key = self.v[x as usize] as usize;
        if !self.keypad.keys[key] {
            self.pc += 2;
        } else {
        }
    }

    // Fx07 - LD Vx, DT
    // Set Vx = delay timer value.
    //
    // The value of DT is placed into Vx.
    fn ld_v_dt(&mut self, x: u8) {
        self.v[x as usize] = self.dt;
    }

    // Fx0A - LD Vx, K
    // Wait for a key press, store the value of the key in Vx.
    //
    // All execution stops until a key is pressed, then the value
    // of that key is stored in Vx.
    fn ld_k(&mut self, x: u8) {
        let mut released = false;

        for (k, v) in self.keypad.keys.iter().enumerate() {
            if self.key_detection_index == 0 {
                if *v {
                    self.key_detection_index = k;
                    self.st = 4;
                    break;
                }
            } else if self.key_detection_index == k && !*v {
                released = true;
                break;
            }
        }

        if released {
            self.v[x as usize] = self.key_detection_index as u8;
            self.key_detection_index = 0;
            return;
        }

        // block until something is pressed
        if self.key_detection_index == 0 {
            self.pc -= 2;
        }
    }

    // Fx15 - LD DT, Vx
    // Set delay timer = Vx.
    //
    // DT is set equal to the value of Vx.
    fn ld_dt_v(&mut self, x: u8) {
        self.dt = self.v[x as usize];
    }

    // Fx18 - LD ST, Vx
    // Set sound timer = Vx.
    //
    // ST is set equal to the value of Vx.
    fn ld_st(&mut self, x: u8) {
        self.st = self.v[x as usize];
    }

    // Fx1E - ADD I, Vx
    // Set I = I + Vx.
    //
    // The values of I and Vx are added, and the results are stored in I.
    fn add_i(&mut self, x: u8) {
        self.i = self.i + self.v[x as usize] as u16;
    }

    // Fx29 - LD F, Vx
    // Set I = location of sprite for digit Vx.
    //
    // The value of I is set to the location for the hexadecimal sprite
    // corresponding to the value of Vx.
    fn ld_f(&mut self, x: u8) {
        self.i = (self.v[x as usize] * 5) as u16; // each font char is 5 bytes
    }

    // Fx33 - LD B, Vx
    // Store BCD representation of Vx in memory locations I, I+1, and I+2.
    //
    // The interpreter takes the decimal value of Vx, and places the hundreds
    // digit in memory at location in I, the tens digit at location I+1,
    // and the ones digit at location I+2.
    fn ld_b(&mut self, x: u8) {
        let v = self.v[x as usize];
        let hundreds = v / 100;
        let tens = v % 100 / 10;
        let ones = v % 10;

        let i = self.i as usize;

        self.memory[i] = hundreds;
        self.memory[i + 1] = tens;
        self.memory[i + 2] = ones;
    }

    // Fx55 - LD [I], Vx
    // Store registers V0 through Vx in memory starting at location I.
    //
    // The interpreter copies the values of registers V0 through Vx
    // into memory, starting at the address in I.
    fn ld_i_v(&mut self, x: u8) {
        for idx in 0..=x {
            let v = self.v[idx as usize];
            self.memory[(self.i + idx as u16) as usize] = v;
        }
        // NOTE: modern behaviour is not to increment the i register
        self.i = self.i + x as u16 + 1;
    }

    // Fx65 - LD Vx, [I]
    // Read registers V0 through Vx from memory starting at location I.
    //
    // The interpreter reads values from memory starting at location
    // I into registers V0 through Vx.
    fn ld_v_i(&mut self, x: u8) {
        for idx in 0..=x {
            let m = self.memory[(self.i + idx as u16) as usize];
            self.v[idx as usize] = m;
        }
        // NOTE: modern behaviour is not to increment the i register
        self.i = self.i + x as u16 + 1;
    }

    // Super Chip-48 Instructions
    // 00Cn - SCD nibble
    // 00FB - SCR
    // 00FC - SCL
    // 00FD - EXIT
    // 00FE - LOW
    // 00FF - HIGH
    // Dxy0 - DRW Vx, Vy, 0
    // Fx30 - LD HF, Vx
    // Fx75 - LD R, Vx
    // Fx85 - LD Vx, R
}

struct GameEntry {
    name: String,
    path: PathBuf,
}

const CHIP8_KEYS: [[u8; 4]; 4] = [
    [0x1, 0x2, 0x3, 0xC],
    [0x4, 0x5, 0x6, 0xD],
    [0x7, 0x8, 0x9, 0xE],
    [0xA, 0x0, 0xB, 0xF],
];

const RUN_HZ: u32 = 540;
const TICK_HZ: u32 = 60;
const TICK_DIVISOR: u32 = RUN_HZ / TICK_HZ;
const MAX_STEPS_PER_FRAME: usize = 4096;
const BEEP_FREQ_MIN_HZ: u32 = 80;
const BEEP_FREQ_MAX_HZ: u32 = 2000;

fn is_game_file(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }

    match path.extension().and_then(|ext| ext.to_str()) {
        Some(ext) => {
            let lower = ext.to_ascii_lowercase();
            lower == "ch8" || lower == "c8k"
        }
        None => true,
    }
}

fn collect_games(root: &Path, dir: &Path, out: &mut Vec<GameEntry>) {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_games(root, &path, out);
            continue;
        }

        if !is_game_file(&path) {
            continue;
        }

        let relative = path.strip_prefix(root).unwrap_or(&path);
        out.push(GameEntry {
            name: relative.display().to_string(),
            path,
        });
    }
}

fn discover_games(root: &Path) -> Vec<GameEntry> {
    let mut games = Vec::new();
    collect_games(root, root, &mut games);
    games.sort_by(|a, b| a.name.cmp(&b.name));
    games
}

fn draw_display_bitmap(ui: &mut egui::Ui, display: &[bool; 2048], pixel_size: f32) {
    let pixel_size = pixel_size.max(1.0);
    let desired = egui::vec2(64.0 * pixel_size, 32.0 * pixel_size);
    let (rect, _) = ui.allocate_exact_size(desired, egui::Sense::hover());
    let painter = ui.painter_at(rect);

    painter.rect_filled(rect, 0.0, Color32::from_rgb(12, 12, 12));

    for y in 0..32 {
        for x in 0..64 {
            let idx = y * 64 + x;
            if display[idx] {
                let pixel_rect = egui::Rect::from_min_size(
                    rect.min + egui::vec2(x as f32 * pixel_size, y as f32 * pixel_size),
                    egui::vec2(pixel_size, pixel_size),
                );
                painter.rect_filled(pixel_rect, 0.0, Color32::from_rgb(230, 230, 230));
            }
        }
    }
}

fn render_hex_view(
    ui: &mut egui::Ui,
    data: &[u8],
    base_addr: usize,
    highlight_addrs: &[usize],
    rom_range: Option<(usize, usize)>,
    fontset_range: (usize, usize),
    id_salt: &'static str,
    max_height: f32,
) {
    let rom_color = Color32::from_rgb(180, 90, 255);
    let fontset_color = Color32::from_rgb(255, 191, 0);
    ScrollArea::vertical()
        .id_salt(id_salt)
        .max_height(max_height)
        .show(ui, |ui| {
            for (row_idx, chunk) in data.chunks(16).enumerate() {
                let row_addr = base_addr + row_idx * 16;
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(format!("{row_addr:03X}:"))
                            .monospace()
                            .color(Color32::LIGHT_BLUE),
                    );
                    for (col, byte) in chunk.iter().enumerate() {
                        let addr = row_addr + col;
                        let is_fontset = addr >= fontset_range.0 && addr < fontset_range.1;
                        let is_rom = rom_range
                            .map(|(start, end)| addr >= start && addr < end)
                            .unwrap_or(false);

                        let mut text = RichText::new(format!("{byte:02X}")).monospace();
                        if is_fontset {
                            text = text.color(fontset_color);
                        } else if is_rom {
                            text = text.color(rom_color);
                        }
                        if highlight_addrs.contains(&addr) {
                            text = text.background_color(Color32::YELLOW).color(Color32::BLACK);
                        }
                        ui.label(text);
                    }
                });
            }
        });
}

fn render_registers(ui: &mut egui::Ui, cpu: &Cpu) {
    ui.label(RichText::new("Registers").strong());
    let opcode = cpu
        .current_opcode()
        .map(|op| format!("{op:04X}"))
        .unwrap_or_else(|| "----".to_string());
    let keyboard_mask = cpu
        .keypad
        .keys
        .iter()
        .enumerate()
        .fold(
            0u16,
            |acc, (idx, pressed)| {
                if *pressed { acc | (1u16 << idx) } else { acc }
            },
        );
    ui.horizontal_top(|ui| {
        ui.vertical(|ui| {
            egui::Grid::new("core_registers_grid")
                .num_columns(2)
                .striped(true)
                .show(ui, |ui| {
                    ui.monospace("PC");
                    ui.monospace(format!("{:03X}", cpu.pc));
                    ui.end_row();

                    ui.monospace("OP");
                    ui.monospace(opcode.clone());
                    ui.end_row();

                    ui.monospace("I");
                    ui.monospace(format!("{:03X}", cpu.i));
                    ui.end_row();

                    ui.monospace("SP");
                    ui.monospace(format!("{:02X}", cpu.sp));
                    ui.end_row();

                    ui.monospace("DT");
                    ui.monospace(format!("{:02X}", cpu.dt));
                    ui.end_row();

                    ui.monospace("ST");
                    ui.monospace(format!("{:02X}", cpu.st));
                    ui.end_row();

                    ui.monospace("VF");
                    ui.monospace(format!("{:02X}", cpu.v[0xf]));
                    ui.end_row();

                    ui.monospace("KBD");
                    ui.monospace(format!("{keyboard_mask:04X}"));
                    ui.end_row();
                });
        });

        ui.add_space(16.0);

        ui.vertical(|ui| {
            ui.label(RichText::new("Vx").strong());
            egui::Grid::new("v_registers_grid")
                .num_columns(8)
                .striped(true)
                .show(ui, |ui| {
                    for idx in 0..16 {
                        ui.monospace(format!("V{idx:X}"));
                        ui.monospace(format!("{:02X}", cpu.v[idx]));
                        if idx % 4 == 3 {
                            ui.end_row();
                        }
                    }
                });
        });
    });
}

#[derive(Clone)]
struct EmulatorSnapshot {
    cpu: Cpu,
    loaded_rom: Vec<u8>,
    loaded_game_name: String,
    running: bool,
    error: Option<String>,
}

struct EmulatorState {
    cpu: Cpu,
    loaded_rom: Vec<u8>,
    loaded_game_name: String,
    running: bool,
    error: Option<String>,
}

impl EmulatorState {
    fn new() -> EmulatorState {
        EmulatorState {
            cpu: Cpu::new(),
            loaded_rom: Vec::new(),
            loaded_game_name: "None".to_string(),
            running: false,
            error: None,
        }
    }

    fn load_game(&mut self, rom: Vec<u8>, name: String) {
        self.running = false;
        self.cpu = Cpu::new();
        self.loaded_rom = rom;
        self.loaded_game_name = name;
        self.cpu.load_game(&self.loaded_rom);
        self.error = None;
    }

    fn reset_current_game(&mut self) {
        self.running = false;
        self.cpu = Cpu::new();
        if !self.loaded_rom.is_empty() {
            self.cpu.load_game(&self.loaded_rom);
        }
    }

    fn run_steps(&mut self, steps: usize) {
        for _ in 0..steps {
            if (self.cpu.pc as usize) >= self.cpu.memory.len().saturating_sub(1) {
                self.running = false;
                self.error = Some(format!("PC out of bounds: 0x{:03X}", self.cpu.pc));
                break;
            }
            self.cpu.execute_cycle();
        }
    }

    fn snapshot(&self) -> EmulatorSnapshot {
        EmulatorSnapshot {
            cpu: self.cpu.clone(),
            loaded_rom: self.loaded_rom.clone(),
            loaded_game_name: self.loaded_game_name.clone(),
            running: self.running,
            error: self.error.clone(),
        }
    }
}

fn spawn_emulator_thread(
    emulator: Arc<Mutex<EmulatorState>>,
    worker_stop: Arc<AtomicBool>,
) -> JoinHandle<()> {
    thread::spawn(move || {
        let tick = Duration::from_secs_f64(1.0 / RUN_HZ as f64);
        let mut next_tick = Instant::now() + tick;

        while !worker_stop.load(Ordering::Relaxed) {
            let now = Instant::now();
            if now < next_tick {
                thread::sleep(next_tick - now);
                continue;
            }

            let mut due_steps = 0usize;
            let now = Instant::now();
            while due_steps < MAX_STEPS_PER_FRAME && now >= next_tick {
                due_steps += 1;
                next_tick += tick;
            }

            if due_steps == MAX_STEPS_PER_FRAME {
                next_tick = Instant::now() + tick;
            }

            if due_steps > 0 {
                let mut emulator = emulator.lock().unwrap_or_else(|poison| poison.into_inner());
                if emulator.running {
                    emulator.run_steps(due_steps);
                }
            }
        }
    })
}

struct Beeper {
    stream: OutputStream,
    sink: Option<Sink>,
    current_frequency_hz: u32,
    current_volume: f32,
}

impl Beeper {
    fn new() -> Result<Beeper, String> {
        let mut stream = OutputStreamBuilder::open_default_stream()
            .map_err(|err| format!("Audio init failed: {err}"))?;
        stream.log_on_drop(false);
        Ok(Beeper {
            stream,
            sink: None,
            current_frequency_hz: 0,
            current_volume: 0.0,
        })
    }

    fn stop(&mut self) {
        if let Some(sink) = self.sink.take() {
            sink.stop();
        }
        self.current_frequency_hz = 0;
        self.current_volume = 0.0;
    }

    fn sync(&mut self, active: bool, frequency_hz: u32, volume: f32) -> Result<(), String> {
        if !active {
            self.stop();
            return Ok(());
        }

        let clamped_frequency = frequency_hz.clamp(BEEP_FREQ_MIN_HZ, BEEP_FREQ_MAX_HZ);
        let clamped_volume = volume.clamp(0.0, 1.0);
        let config_changed = self.current_frequency_hz != clamped_frequency
            || (self.current_volume - clamped_volume).abs() > f32::EPSILON;

        if self.sink.is_none() || config_changed {
            self.stop();
            let sink = Sink::connect_new(self.stream.mixer());
            let tone = SineWave::new(clamped_frequency as f32)
                .amplify(clamped_volume)
                .repeat_infinite();
            sink.append(tone);
            sink.play();
            self.current_frequency_hz = clamped_frequency;
            self.current_volume = clamped_volume;
            self.sink = Some(sink);
        }

        Ok(())
    }
}

struct ChipApp {
    emulator: Arc<Mutex<EmulatorState>>,
    worker_stop: Arc<AtomicBool>,
    worker_handle: Option<JoinHandle<()>>,
    beeper: Option<Beeper>,
    audio_error: Option<String>,
    games: Vec<GameEntry>,
    selected_game: usize,
    jump_steps: usize,
    beep_frequency_hz: u32,
    beep_volume: f32,
    sticky_keypad_buttons: bool,
}

impl Drop for ChipApp {
    fn drop(&mut self) {
        if let Some(beeper) = &mut self.beeper {
            beeper.stop();
        }
        self.worker_stop.store(true, Ordering::Relaxed);
        if let Some(handle) = self.worker_handle.take() {
            let _ = handle.join();
        }
    }
}

impl ChipApp {
    fn new() -> ChipApp {
        let games = discover_games(Path::new("./games"));
        let emulator = Arc::new(Mutex::new(EmulatorState::new()));
        let worker_stop = Arc::new(AtomicBool::new(false));
        let worker_handle = Some(spawn_emulator_thread(
            Arc::clone(&emulator),
            Arc::clone(&worker_stop),
        ));
        let (beeper, audio_error) = match Beeper::new() {
            Ok(beeper) => (Some(beeper), None),
            Err(err) => (None, Some(err)),
        };

        let mut app = ChipApp {
            emulator,
            worker_stop,
            worker_handle,
            beeper,
            audio_error,
            games,
            selected_game: 0,
            jump_steps: 100,
            beep_frequency_hz: 440,
            beep_volume: 0.15,
            sticky_keypad_buttons: false,
        };

        if app.games.is_empty() {
            app.with_emulator_mut(|emulator| {
                emulator.error = Some("No games found under ./games".to_string());
            });
        } else {
            app.reload_selected_game();
        }

        app
    }

    fn sync_audio_for_st(&mut self, st: u8) {
        let wants_beep = st > 0;
        if let Some(beeper) = &mut self.beeper {
            if let Err(err) = beeper.sync(wants_beep, self.beep_frequency_hz, self.beep_volume) {
                self.audio_error = Some(err);
                self.beeper = None;
            }
        }
    }

    fn with_emulator_mut<R>(&self, f: impl FnOnce(&mut EmulatorState) -> R) -> R {
        let mut emulator = self
            .emulator
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        f(&mut emulator)
    }

    fn emulator_snapshot(&self) -> EmulatorSnapshot {
        let emulator = self
            .emulator
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        emulator.snapshot()
    }

    fn selected_game_name(&self) -> String {
        self.games
            .get(self.selected_game)
            .map(|g| g.name.clone())
            .unwrap_or_else(|| "No games".to_string())
    }

    fn reload_selected_game(&mut self) {
        let Some(path) = self.games.get(self.selected_game).map(|g| g.path.clone()) else {
            self.with_emulator_mut(|emulator| {
                *emulator = EmulatorState::new();
                emulator.error = Some("No games available to load".to_string());
            });
            return;
        };

        match fs::read(&path) {
            Ok(rom) => {
                let name = self
                    .games
                    .get(self.selected_game)
                    .map(|g| g.name.clone())
                    .unwrap_or_else(|| path.display().to_string());
                self.with_emulator_mut(|emulator| {
                    emulator.load_game(rom, name);
                });
            }
            Err(err) => {
                self.with_emulator_mut(|emulator| {
                    *emulator = EmulatorState::new();
                    emulator.error = Some(format!("Failed to load {}: {err}", path.display()));
                });
            }
        }
    }
}

impl eframe::App for ChipApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.request_repaint_after(Duration::from_millis(16));

        let dropped_files = ctx.input(|i| i.raw.dropped_files.clone());
        let hovered_files = ctx.input(|i| i.raw.hovered_files.clone());

        let snapshot = self.emulator_snapshot();
        let mut toggle_running = false;
        let mut step_once = false;
        let mut reset = false;
        let mut jump_requested = false;
        let mut release_all_keys = false;
        let mut toggled_keys: Vec<u8> = Vec::new();
        let mut momentary_keys: Vec<(u8, bool)> = Vec::new();
        let mut clear_keys_for_mode_switch = false;

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.columns(2, |columns| {
                columns[0].heading("CHIP-8 Debugger");
                columns[0].horizontal_top(|ui| {
                    ui.vertical(|ui| {
                        ui.label(RichText::new("Game + Execution").strong());
                        if !self.games.is_empty() {
                            let mut next_selected = self.selected_game;
                            egui::ComboBox::from_label("Game")
                                .selected_text(self.selected_game_name())
                                .show_ui(ui, |ui| {
                                    for (idx, game) in self.games.iter().enumerate() {
                                        ui.selectable_value(&mut next_selected, idx, &game.name);
                                    }
                                });
                            if next_selected != self.selected_game {
                                self.selected_game = next_selected;
                                self.reload_selected_game();
                            }
                        } else {
                            ui.label("No game binaries discovered.");
                        }

                        ui.horizontal(|ui| {
                            let run_label = if snapshot.running { "Stop" } else { "Start" };
                            if ui.button(run_label).clicked() {
                                toggle_running = true;
                            }
                            if ui.button("Step").clicked() {
                                step_once = true;
                            }
                            if ui.button("Reset").clicked() {
                                reset = true;
                            }
                        });
                        ui.horizontal(|ui| {
                            ui.add(
                                egui::DragValue::new(&mut self.jump_steps)
                                    .range(1..=500_000)
                                    .speed(1.0)
                                    .prefix("N="),
                            );
                            if ui.button("Jump N").clicked() {
                                jump_requested = true;
                            }
                        });

                        ui.label(format!("Run rate: {RUN_HZ} Hz"));
                        ui.label(format!("Timer tick: {TICK_HZ} Hz"));
                        ui.label(format!(
                            "Execution: {}",
                            if snapshot.running {
                                "Running"
                            } else {
                                "Stopped"
                            }
                        ));
                    });

                    ui.add_space(16.0);

                    ui.vertical(|ui| {
                        ui.label(RichText::new("Audio").strong());
                        ui.horizontal(|ui| {
                            ui.label("Tone");
                            ui.add(
                                egui::DragValue::new(&mut self.beep_frequency_hz)
                                    .range(BEEP_FREQ_MIN_HZ..=BEEP_FREQ_MAX_HZ)
                                    .speed(1.0)
                                    .suffix(" Hz"),
                            );
                        });
                        ui.add(
                            egui::Slider::new(&mut self.beep_volume, 0.0..=1.0).text("Beep volume"),
                        );
                        if let Some(audio_error) = &self.audio_error {
                            ui.colored_label(Color32::LIGHT_RED, audio_error);
                        }
                    });
                });

                if let Some(error) = &snapshot.error {
                    columns[0].colored_label(Color32::LIGHT_RED, error);
                }

                columns[0].separator();
                columns[0].label("Display bitmap");
                draw_display_bitmap(&mut columns[0], &snapshot.cpu.display.memory, 8.0);

                columns[0].separator();
                columns[0].label("CHIP-8 keypad");
                if columns[0]
                    .checkbox(&mut self.sticky_keypad_buttons, "Sticky keypad buttons")
                    .changed()
                    && !self.sticky_keypad_buttons
                {
                    clear_keys_for_mode_switch = true;
                }
                for row in CHIP8_KEYS {
                    columns[0].horizontal(|ui| {
                        for key in row {
                            let pressed = snapshot.cpu.keypad.keys[key as usize];
                            let fill = if pressed {
                                Color32::from_rgb(50, 130, 60)
                            } else {
                                ui.style().visuals.widgets.inactive.bg_fill
                            };
                            let text =
                                RichText::new(format!("{key:X}"))
                                    .monospace()
                                    .color(if pressed {
                                        Color32::WHITE
                                    } else {
                                        ui.visuals().text_color()
                                    });
                            let response =
                                ui.add_sized([34.0, 28.0], egui::Button::new(text).fill(fill));
                            if self.sticky_keypad_buttons {
                                if response.clicked() {
                                    toggled_keys.push(key);
                                }
                            } else {
                                let down = response.is_pointer_button_down_on();
                                momentary_keys.push((key, down));
                            }
                        }
                    });
                }
                if columns[0].button("Release all keys").clicked() {
                    release_all_keys = true;
                }

                columns[0].separator();
                render_registers(&mut columns[0], &snapshot.cpu);

                columns[1].heading("Hex Views");
                columns[1].label(format!("Loaded game: {}", snapshot.loaded_game_name));
                columns[1].label(format!("ROM bytes: {}", snapshot.loaded_rom.len()));
                columns[1].label("Legend: ROM bytes are purple, font bytes are amber");
                columns[1].label(
                    RichText::new(format!("Memory (PC -> 0x{:03X})", snapshot.cpu.pc)).strong(),
                );
                let pc = snapshot.cpu.pc as usize;
                let opcode_bytes = [pc, pc.saturating_add(1)];
                let rom_start = 0x200usize;
                let rom_end = rom_start + snapshot.loaded_rom.len();
                render_hex_view(
                    &mut columns[1],
                    &snapshot.cpu.memory,
                    0x000,
                    &opcode_bytes,
                    Some((rom_start, rom_end)),
                    (0, FONTSET.len()),
                    "memory_hex",
                    840.0,
                );
            });
        });

        if !hovered_files.is_empty() {
            egui::Window::new("Drop ROM")
                .anchor(egui::Align2::CENTER_TOP, [0.0, 20.0])
                .resizable(false)
                .collapsible(false)
                .title_bar(false)
                .show(ctx, |ui| {
                    ui.strong("Drop game file to load");
                });
        }

        if !dropped_files.is_empty() {
            let mut dropped_rom: Option<(Vec<u8>, String)> = None;
            let mut dropped_error: Option<String> = None;

            for file in dropped_files {
                if let Some(path) = file.path {
                    match fs::read(&path) {
                        Ok(bytes) => {
                            let name = path
                                .file_name()
                                .and_then(|n| n.to_str())
                                .map(ToString::to_string)
                                .unwrap_or_else(|| path.display().to_string());
                            dropped_rom = Some((bytes, name));
                            break;
                        }
                        Err(err) => {
                            dropped_error = Some(format!(
                                "Failed to read dropped file {}: {err}",
                                path.display()
                            ));
                        }
                    }
                    continue;
                }

                if let Some(bytes) = file.bytes {
                    if bytes.is_empty() {
                        dropped_error = Some("Dropped file is empty".to_string());
                        continue;
                    }
                    let name = if file.name.is_empty() {
                        "Dropped ROM".to_string()
                    } else {
                        file.name
                    };
                    dropped_rom = Some((bytes.as_ref().to_vec(), name));
                    break;
                }

                dropped_error = Some("Dropped file has no readable path or bytes".to_string());
            }

            if let Some((rom, name)) = dropped_rom {
                self.with_emulator_mut(|emulator| {
                    emulator.load_game(rom, name);
                });
            } else if let Some(err) = dropped_error {
                self.with_emulator_mut(|emulator| {
                    emulator.error = Some(err);
                });
            }
        }

        if toggle_running
            || step_once
            || reset
            || jump_requested
            || release_all_keys
            || !toggled_keys.is_empty()
            || !momentary_keys.is_empty()
            || clear_keys_for_mode_switch
        {
            let jump_steps = self.jump_steps;
            self.with_emulator_mut(|emulator| {
                if toggle_running {
                    emulator.running = !emulator.running;
                }
                if step_once {
                    emulator.running = false;
                    emulator.run_steps(1);
                }
                if reset {
                    emulator.reset_current_game();
                }
                if jump_requested {
                    emulator.running = false;
                    emulator.run_steps(jump_steps);
                }
                if release_all_keys {
                    emulator.cpu.keypad.keys = [false; 16];
                }
                if clear_keys_for_mode_switch {
                    emulator.cpu.keypad.keys = [false; 16];
                }
                for key in toggled_keys {
                    let idx = key as usize;
                    emulator.cpu.keypad.keys[idx] = !emulator.cpu.keypad.keys[idx];
                }
                for (key, down) in momentary_keys {
                    let idx = key as usize;
                    emulator.cpu.keypad.keys[idx] = down;
                }
            });
        }

        let st = self.emulator_snapshot().cpu.st;
        self.sync_audio_for_st(st);
    }
}

fn main() -> eframe::Result<()> {
    env_logger::init();
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1500.0, 900.0]),
        ..Default::default()
    };

    eframe::run_native(
        "CHIP-8 Emulator",
        native_options,
        Box::new(|_cc| Ok(Box::new(ChipApp::new()))),
    )
}
