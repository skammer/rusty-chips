#![feature(advanced_slice_patterns, slice_patterns)]

// extern crate ggez;
use std::error::Error;
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

pub fn read_game(path: &str) -> Vec<u8> {
    let path = Path::new(path);
    let display = path.display();

    let mut file = match File::open(path) {
        Err(why) => panic!("Couldn't open file {}: {}", display, Error::description(&why)),
        Ok(file) => file,
    };

    let mut game = Vec::new();
    match file.read_to_end(&mut game) {
        Err(why) => panic!("Couldn't read file {}: {}", display, Error::description(&why)),
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

      // let opcode_string: String = format!("{:x}", opcode);
      // let separate_bytes: Vec<char> = opcode_string.chars().collect(); //.map(|c| c as u8).collect();
      let separate_bytes = self.split_u4(opcode);

      print_binary(&separate_bytes);

      let kk = opcode & 0x00ff;
      let nnn = opcode & 0x0fff;

      // TODO: move high/low byte trimming here, instead of passing opcode

      match separate_bytes[..] {
        [0, 0, 0xEu8, 0] => self.cls(),
        [0, 0, 0xEu8, 0xEu8] => self.ret(),
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
        _ => println!("Something random")
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
        }
    }

    // 4xkk - SNE Vx, byte
    // Skip next instruction if Vx != kk.
    // The interpreter compares register Vx to kk, and if they are not equal, increments the 
    // program counter by 2.
    fn sen(&mut self, x: u8, kk: u16) {
        if self.v[x as usize] as u16 != kk  {
            self.pc += 2;
        }
    }

    // 5xy0 - SE Vx, Vy
    // Skip next instruction if Vx = Vy.
    // The interpreter compares register Vx to register Vy, and if they are equal, increments the 
    // program counter by 2.
    fn sexy(&mut self, x: u8, y: u8) {
        if self.v[x as usize] == self.v[y as usize] {
            self.pc += 2;
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
}



fn main() {
    let mut cpu: Cpu = Cpu::new();

    println!("Game");

    let game: Vec<u8> = read_game("./games/PONG");

    println!("Memory");

    cpu.load_game(game);

    cpu.execute_cycle();

    println!("Running CHIP-8 CPU");
    println!("timers running at 60hz");
}
