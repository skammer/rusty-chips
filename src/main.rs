// use rand::Rng;

extern crate rand;
use rand::Rng;
use rand::distributions::{IndependentSample, Range};

// extern crate ggez;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;

pub fn print_binary(bytes: &Vec<u8>)
{
    for x in bytes.iter() {
        print!("{:x}", x)
    }
    println!("");
}


pub fn print_display(bytes: &Vec<bool>)
{
    print!("---------------------------------------\n");
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
    print!("\n---------------------------------------");
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

pub struct Keypad {
    pub keys: [bool; 16]
}

impl Keypad {
    pub fn new() -> Keypad {
        let keypad = Keypad {
            keys: [false; 16]
        };

        keypad
    }
}

pub struct Display {
    pub memory: [bool; 2048]
}

impl Display {
    pub fn new() -> Display {
        let display = Display {
            memory: [false; 2048]
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
    0xF0, 0x80, 0xF0, 0x80, 0x80  // F
];

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
    // overflow flag
    pub vf: u8
}


fn read_word(memory: [u8; 4096], counter: u16) -> u16 {
    (memory[counter as usize] as u16) << 8
        | (memory[(counter + 1) as usize] as u16)
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
            vf: 0,
            keypad: Keypad::new(),
            display: Display::new()
        };

        cpu.memory[..FONTSET.len()].clone_from_slice(&FONTSET);

        // print_binary(&cpu.memory.to_vec());

        cpu
    }

    pub fn load_game(&mut self, game: Vec<u8>) {
        let start = 512;
        let end = start + game.len();
        self.memory[start..end].clone_from_slice(&game);

        // print_binary(&self.memory.to_vec());
    }

    pub fn execute_cycle(&mut self) {
        println!("{}",  self.pc);
        let opcode: u16 = read_word(self.memory, self.pc);
        self.process_opcode(opcode);
    }

    pub fn split_u4(&mut self, opcode: u16) -> Vec<u8> {
        let u4_0: u8 = ((opcode & 0xf000) >> 12) as u8;
        let u4_1: u8 = ((opcode & 0x0f00) >> 8) as u8;
        let u4_2: u8 = ((opcode & 0x00f0) >> 4) as u8;
        let u4_3: u8 = (opcode & 0x000f) as u8;

        vec!(u4_0, u4_1, u4_2, u4_3)
    }

    fn process_opcode(&mut self, opcode: u16) {
      println!("processing opcode, {:x}", opcode);

      let separate_bytes = self.split_u4(opcode);

      print_binary(&separate_bytes);

      let kk = opcode & 0x00ff;
      let nnn = opcode & 0x0fff;
      let n = opcode & 0x000f;

      match separate_bytes[..] {
        [0,   0, 0xE, 0]   => self.cls(),
        [0,   0, 0xE, 0xE] => self.ret(),
        [0,   _, _,   _]   => self.sys(),
        [1,   _, _,   _]   => self.jp(nnn),
        [2,   _, _,   _]   => self.call(nnn),
        [3,   x, _,   _]   => self.se(x, kk),
        [4,   x, _,   _]   => self.sen(x, kk),
        [5,   x, y,   0]   => self.sexy(x, y),
        [6,   x, _,   _]   => self.ldxkk(x, kk),
        [7,   x, _,   _]   => self.addxkk(x, kk),
        [8,   x, y,   0]   => self.ldxy(x, y),
        [8,   x, y,   1]   => self.or(x, y),
        [8,   x, y,   2]   => self.and(x, y),
        [8,   x, y,   3]   => self.xor(x, y),
        [8,   x, y,   4]   => self.add(x, y),
        [8,   x, y,   5]   => self.sub(x, y),
        [8,   x, y,   6]   => self.shr(x, y),
        [8,   x, y,   7]   => self.subn(x, y),
        [8,   x, y,   0xE] => self.shl(x, y),
        [9,   x, y,   0]   => self.sne(x, y),
        [0xA, _, _,   _]   => self.ldi(nnn),
        [0xB, _, _,   _]   => self.jpv0(nnn),
        [0xC, x, _,   _]   => self.rnd(x, kk),
        [0xD, x, y,   _]   => self.drw(x, y, n as u8),
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
        _ => println!("Unimplemented opcode: {}", opcode)
      }
    }

    //
    // Opcodes
    //

    // 0nnn - SYS addr
    // Jump to a machine code routine at nnn.
    // This instruction is only used on the old computers on which Chip-8 was originally
    // implemented. It is ignored by modern interpreters.
    fn sys(&mut self) {
        self.pc += 1;
    }

    // 00E0 - CLS
    // Clear the display.
    fn cls(&mut self) {
        self.display.clear();
        self.pc += 1;
    }

    // 00EE - RET
    // Return from a subroutine.
    // The interpreter sets the program counter to the address at the top of the stack, then
    // subtracts 1 from the stack pointer.
    fn ret(&mut self) {
        self.pc = *self.stack.get(self.sp as usize).unwrap();
        if self.sp > 0 {
            self.sp -= 1;
        }
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
        self.sp += 1;
        self.stack[self.sp as usize] = self.pc;
        self.pc = nnn;
    }

    // 3xkk - SE Vx, byte
    // Skip next instruction if Vx = kk.
    // The interpreter compares register Vx to kk, and if they are equal, increments the program
    // counter by 2.
    fn se(&mut self, x: u8, kk: u16) {
        if self.v[x as usize] as u16 == kk  {
            self.pc += 2;
        } else {
            self.pc += 1;
        }
    }

    // 4xkk - SNE Vx, byte
    // Skip next instruction if Vx != kk.
    // The interpreter compares register Vx to kk, and if they are not equal, increments the
    // program counter by 2.
    fn sen(&mut self, x: u8, kk: u16) {
        if self.v[x as usize] as u16 != kk  {
            self.pc += 2;
        } else {
            self.pc += 1;
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
            self.pc += 1;
        }
    }

    // 6xkk - LD Vx, byte
    // Set Vx = kk.
    // The interpreter puts the value kk into register Vx.
    fn ldxkk(&mut self, x: u8, kk: u16) {
        self.v[x as usize] = kk as u8;
        self.pc += 1;
    }

    // 7xkk - ADD Vx, byte
    // Set Vx = Vx + kk.
    // Adds the value kk to the value of register Vx, then stores the result in Vx.
    fn addxkk(&mut self, x: u8, kk: u16) {
        self.v[x as usize] += kk as u8;
        self.pc += 1;
    }

    // 8xy0 - LD Vx, Vy
    // Set Vx = Vy.
    // Stores the value of register Vy in register Vx.
    fn ldxy(&mut self, x: u8, y: u8) {
        self.v[x as usize] = self.v[y as usize];
        self.pc += 1;
    }

    // 8xy1 - OR Vx, Vy
    // Set Vx = Vx OR Vy.
    // Performs a bitwise OR on the values of Vx and Vy, then stores the result in Vx.
    fn or(&mut self, x: u8, y: u8) {
        let res: u8 = self.v[x as usize] | self.v[y as usize];
        self.v[x as usize] = res;
        self.pc += 1;
    }

    // 8xy2 - AND Vx, Vy
    // Set Vx = Vx AND Vy.
    // Performs a bitwise AND on the values of Vx and Vy, then stores the result in Vx.
    fn and(&mut self, x: u8, y: u8) {
        let res: u8 = self.v[x as usize] & self.v[y as usize];
        self.v[x as usize] = res;
        self.pc += 1;
    }

    // 8xy3 - XOR Vx, Vy
    // Set Vx = Vx XOR Vy.
    // Performs a bitwise exclusive OR on the values of Vx and Vy, then stores the result in Vx.
    fn xor(&mut self, x: u8, y: u8) {
        let res: u8 = self.v[x as usize] ^ self.v[y as usize];
        self.v[x as usize] = res;
        self.pc += 1;
    }

    // 8xy4 - ADD Vx, Vy
    // Set Vx = Vx + Vy, set VF = carry.
    // The values of Vx and Vy are added together. If the result is greater than 8 bits (i.e.,
    // > 255,) VF is set to 1, otherwise 0. Only the lowest 8 bits of the result are kept, and
    // stored in Vx.
    fn add(&mut self, x: u8, y: u8) {
        let vx = self.v[x as usize];
        let res: u8 = vx.wrapping_add(self.v[y as usize]);
        let carry: u8 = if res >= vx { 0 } else { 1 };

        self.v[x as usize] = res;
        self.vf = carry;
        self.pc += 1;
    }

    // 8xy5 - SUB Vx, Vy
    // Set Vx = Vx - Vy, set VF = NOT borrow.
    // If Vx > Vy, then VF is set to 1, otherwise 0. Then Vy is subtracted from Vx, and the results
    // stored in Vx.
    fn sub(&mut self, x: u8, y: u8) {
        let vx = self.v[x as usize];
        let vy = self.v[y as usize];
        let res: u8 = vx.wrapping_sub(vy);
        let carry: u8 = if vx > vy { 1 } else { 0 };

        self.v[x as usize] = res;
        self.vf = carry;
        self.pc += 1;
    }

    // 8xy6 - SHR Vx {, Vy}
    // Set Vx = Vx SHR 1.
    // If the least-significant bit of Vx is 1, then VF is set to 1, otherwise 0.  Then Vx is
    // divided by 2.
    fn shr(&mut self, x: u8, _y: u8) {
        let vx = self.v[x as usize];
        let carry: u8 = vx >> 3;

        self.v[x as usize] = vx >> 1;
        self.vf = carry;
        self.pc += 1;
    }

    // 8xy7 - SUBN Vx, Vy
    // Set Vx = Vy - Vx, set VF = NOT borrow.
    // If Vy > Vx, then VF is set to 1, otherwise 0. Then Vx is subtracted from Vy, and the results
    // stored in Vx.
    fn subn(&mut self, x: u8, y: u8) {
        let vx = self.v[x as usize];
        let vy = self.v[y as usize];
        let res: u8 = vy.wrapping_sub(vx);
        let carry: u8 = if vy > vx { 1 } else { 0 };

        self.v[x as usize] = res;
        self.vf = carry;
        self.pc += 1;
    }

    // 8xyE - SHL Vx {, Vy}
    // Set Vx = Vx SHL 1.
    // If the most-significant bit of Vx is 1, then VF is set to 1, otherwise to
    // 0. Then Vx is multiplied by 2.
    fn shl(&mut self, x: u8, _y: u8) {
        let vx = self.v[x as usize];
        let carry: u8 = vx >> 3;

        self.v[x as usize] = vx << 1;
        self.vf = carry;
        self.pc += 1;
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
            self.pc += 1;
        }
    }

    // Annn - LD I, addr
    // Set I = nnn.
    // The value of register I is set to nnn.
    fn ldi(&mut self, nnn: u16) {
        self.i = nnn;
        self.pc += 1;
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
        self.pc += 1;
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
        // Start with no collision
        self.vf = 0;

        let x_pos = self.v[x as usize] as u16;
        let y_pos = self.v[y as usize] as u16;

        // For each row of the sprite
        for row in 0..n {
            // Get sprite byte from memory at I + row
            let sprite_byte = self.memory[(self.i + row as u16) as usize];

            // For each pixel in the row (8 pixels per byte)
            for pixel in 0..8u16 {
                // Check if this pixel is set in the sprite
                if (sprite_byte & (0x80 >> pixel)) != 0 {
                    // Calculate screen position with wraparound
                    // Screen is 64x32, so X wraps at 64, Y wraps at 32
                    let screen_x = (x_pos + pixel) % 64;
                    let screen_y = (y_pos + row as u16) % 32;
                    let idx = (screen_y * 64 + screen_x) as usize;

                    // XOR pixel onto screen, check for collision
                    if self.display.memory[idx] {
                        self.vf = 1;
                    }
                    self.display.memory[idx] ^= true;
                }
            }
        }

        self.pc += 1;
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
            self.pc += 1;
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
            self.pc += 1;
        }
    }

    // Fx07 - LD Vx, DT
    // Set Vx = delay timer value.
    //
    // The value of DT is placed into Vx.
    fn ld_v_dt(&mut self, x: u8) {
        self.v[x as usize] = self.dt;
        self.pc += 1;
    }

    // Fx0A - LD Vx, K
    // Wait for a key press, store the value of the key in Vx.
    //
    // All execution stops until a key is pressed, then the value
    // of that key is stored in Vx.
    fn ld_k(&mut self, x: u8) {
        'outer: loop {
            for (k, v) in self.keypad.keys.iter().enumerate() {
                if *v {
                    self.v[x as usize] = k as u8;
                    self.pc += 1;
                    break 'outer;
                }
            }
        }
    }

    // Fx15 - LD DT, Vx
    // Set delay timer = Vx.
    //
    // DT is set equal to the value of Vx.
    fn ld_dt_v(&mut self, x: u8) {
        self.dt = self.v[x as usize];
        self.pc += 1;
    }

    // Fx18 - LD ST, Vx
    // Set sound timer = Vx.
    //
    // ST is set equal to the value of Vx.
    fn ld_st(&mut self, x: u8) {
        self.st = self.v[x as usize];
        self.pc += 1;
    }

    // Fx1E - ADD I, Vx
    // Set I = I + Vx.
    //
    // The values of I and Vx are added, and the results are stored in I.
    fn add_i(&mut self, x: u8) {
        self.i = self.i + self.v[x as usize] as u16;
        self.pc += 1;
    }

    // Fx29 - LD F, Vx
    // Set I = location of sprite for digit Vx.
    //
    // The value of I is set to the location for the hexadecimal sprite
    // corresponding to the value of Vx.
    fn ld_f(&mut self, x: u8) {
        self.i = (self.v[x as usize] * 5) as u16; // each font char is 5 bytes
        self.pc += 1;
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

        self.memory[i]   = hundreds;
        self.memory[i+1] = tens;
        self.memory[i+2] = ones;

        self.pc += 1;
    }

    // Fx55 - LD [I], Vx
    // Store registers V0 through Vx in memory starting at location I.
    //
    // The interpreter copies the values of registers V0 through Vx into memory, starting at the address in I.
    fn ld_i_v(&mut self, x: u8) {

        for idx in 0..x {
            let v = self.v[idx as usize];
            self.memory[(self.i + idx as u16) as usize] = v;
        }

        self.pc += 1;
    }

    // Fx65 - LD Vx, [I]
    // Read registers V0 through Vx from memory starting at location I.
    //
    // The interpreter reads values from memory starting at location
    // I into registers V0 through Vx.
    fn ld_v_i(&mut self, x: u8) {

        for idx in 0..x {
            let m = self.memory[(self.i + idx as u16) as usize];
            self.v[idx as usize] = m;
        }

        self.pc += 1;
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

fn main() {
    let mut cpu: Cpu = Cpu::new();

    println!("Game");

    // let game: Vec<u8> = read_game("./games/PONG");
    let game: Vec<u8> = read_game("./games/1-chip8-logo.ch8");

    println!("Memory");

    // print_binary(&cpu.memory.to_vec());
    cpu.load_game(game);
    // print_binary(&cpu.memory.to_vec());


    for i in 0..10 {
        cpu.execute_cycle();
    }


    print_display(&cpu.display.memory.to_vec());

    println!("Running CHIP-8 CPU");
    println!("timers running at 60hz");
}
