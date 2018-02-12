// #![feature(advanced_slice_patterns, slice_patterns)]

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
    pub st: u8
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

      // println!("{:?}", separate_bytes);
      print_binary(&separate_bytes);

      match separate_bytes {
        &['6', _, _, _] => println!("LD Vx, byte"),
        _ => println!("Something random")
      }
    }

    fn 
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
