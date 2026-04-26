use std::ops::Deref;
use std::thread::current;

use rand::Rng;
use rand::distr::StandardUniform;
use rand::rngs::ThreadRng;

// =================================
// Fontset for Chip8
// =================================
const FONTSET: [u8; 80] = [
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

// VF register index
const REG_V0: usize = 0;
const REG_VF: usize = 0xF;
const ADDRESS_BITS: u16 = 12;
const MAX_ADDRESS: u16 = (1 << ADDRESS_BITS) - 1;
const SIZE_OF_SPRITE: u16 = 5;
const DISPLAY_HEIGHT: usize = 32;
const DISPLAY_WIDTH: usize = 64;

// =================================
// Useful macros
// =================================
macro_rules! extract_bits {
    ($num:expr, $shift:expr, $mask:expr) => {
        (($num >> $shift) & $mask)
    };
}

macro_rules! reg_x {
    ($num: expr) => {
        extract_bits!($num, 8, 0xF) as usize
    };
}

macro_rules! reg_y {
    ($num: expr) => {
        extract_bits!($num, 4, 0xF) as usize
    };
}

// =================================
// Implementation of Chip8
// =================================

#[cfg_attr(test, derive(Clone, Debug))]
struct Chip8 {
    // Registers
    registers: [u8; 16],
    pc: u16,
    index: u16,
    timer_delay: u8,
    timer_sound: u8,

    // Memory
    memory: [u8; 4096],
    stack: [u16; 16],
    sp: u16,

    // I/O
    graphics: [u8; DISPLAY_HEIGHT * DISPLAY_WIDTH],
    keypad: [u8; 16],

    // Utils
    rng: ThreadRng,
}

impl Chip8 {
    // Creating a new chip8 instance
    //TODO: Move chip instance to heap instead of stack
    fn new() -> Chip8 {
        return Chip8 {
            registers: [0; 16],
            pc: 0x200,
            index: 0,
            timer_delay: 0,
            timer_sound: 0,
            memory: [0; 4096],
            stack: [0; 16],
            sp: 0,
            graphics: [0; DISPLAY_HEIGHT * DISPLAY_WIDTH],
            keypad: [0; 16],

            rng: rand::thread_rng(),
        };
    }

    // Init/Reset a chip8
    fn init(&mut self, program: &[u16]) {
        // Set reset all values
        self.registers = [0; 16];
        self.pc = 0x200;
        self.index = 0;
        self.timer_delay = 0;
        self.timer_sound = 0;
        self.memory = [0; 4096];
        self.stack = [0; 16];
        self.sp = 0;
        self.graphics = [0; 64 * 32];
        self.keypad = [0; 16];

        // Load fontset into memory
        for i in 0..80 {
            self.memory[i] = FONTSET[i];
        }

        // Load program into memory
        let mut memory_pos = self.pc;
        for opcode in program {
            let low = (opcode & 0x00FF) as u8;
            let high = extract_bits!(opcode, 8, 0xFF) as u8;
            self.memory[memory_pos as usize] = high;
            self.memory[(memory_pos + 1) as usize] = low;

            memory_pos += 2;
        }

        dbg!(self.memory[self.pc as usize]);
    }

    // Emulating one CPU cycle
    fn emulateCycle(&mut self) {
        // Fetch opcode
        let opcode: u16 = u16::from_be_bytes([
            self.memory[self.pc as usize],
            self.memory[self.pc as usize + 1],
        ]);

        // Increment pc directly in order to avoid confusion at jumps
        self.pc += 2;

        match (opcode & 0xF000) {
            0x0000 => match opcode {
                0x00E0 => {
                    self._opcode_00E0();
                } // Clear screen
                0x00EE => {
                    self._opcode_00EE();
                } // Return from subroutine
                _ => {
                    self._opcode_0NNN(opcode);
                } // Execute machine language subroutine at NNN
            },

            0x1000 => {
                self._opcode_1NNN(opcode);
            } // Jump to address NNN
            0x2000 => {
                self._opcode_2NNN(opcode);
            } // Execute subroutine at NNN
            0x3000 => {
                self._opcode_3XNN(opcode);
            } // Skip the following instruction in value of VX equals NN
            0x4000 => {
                self._opcode_4XNN(opcode);
            } // Skip the following instruction if value of VX not equal NN
            0x5000 => {
                self._opcode_5XY0(opcode);
            } // Skip the following instruction if value in VX equal to value in VY
            0x6000 => {
                self._opcode_6XNN(opcode);
            } // Store number NN in VX
            0x7000 => {
                self._opcode_7XNN(opcode);
            } // Add value NN to VX
            0x8000 => match (opcode & 0x000F) {
                0x0 => {
                    self._opcode_8XY0(opcode);
                } // Set VX to VY
                0x1 => {
                    self._opcode_8XY1(opcode);
                } // Set VX to VX OR VY
                0x2 => {
                    self._opcode_8XY2(opcode);
                } // Set VX to VX AND VY
                0x3 => {
                    self._opcode_8XY3(opcode);
                } // Set VX to VX XOR VY
                0x4 => {
                    self._opcode_8XY4(opcode);
                } // Add the value of VY to VX (VF = 1 if carry otherwise 0)
                0x5 => {
                    self._opcode_8XY5(opcode);
                } // Subtract VY from VX (VF = 1 if borrow occurs, otherwise 0)
                0x6 => {
                    self._opcode_8XY6(opcode);
                } // Shift VY right 1 bit, store in VX (VF = LSB prior to shift)
                0x7 => {
                    self._opcode_8XY7(opcode);
                } // Set VX to VY minus VX (VF = 1 if borrow occurs)
                0xE => {
                    self._opcode_8XYE(opcode);
                } // Shift VY left 1 bit, store in VX (VF = MSB prior to shift)
                _ => panic!("Unknown opcode"),
            },
            0x9000 => {
                self._opcode_9XY0(opcode);
            } // Skip instruction if VX and VY not equal
            0xA000 => {
                self._opcode_ANNN(opcode);
            } // Store memory address NNN in I
            0xB000 => {
                self._opcode_BNNN(opcode);
            } // Jump to address NNN + V0
            0xD000 => {
                self._opcode_DXYN(opcode);
            } // Draw sprite
            0xE000 => match (opcode & 0x00FF) {
                0x9E => {
                    self._opcode_EX9E(opcode);
                } // Skip instruction if key in VX pressed
                0xA1 => {
                    self._opcode_EXA1(opcode);
                } // Skip instruction if key in VX not pressed
                _ => panic!("Unknown opcode"),
            },

            0xF000 => match (opcode & 0x00FF) {
                0x07 => {
                    self._opcode_FX07(opcode);
                } // Store the current delay in register VX
                0x0A => {
                    self._opcode_FX0A(opcode);
                } // Wait for keypress, store result in VX
                0x15 => {
                    self._opcode_FX0A(opcode);
                } // Set delay timer to VX
                0x18 => {
                    self._opcode_FX18(opcode);
                } // Set sound timer to VX
                0x1E => {
                    self._opcode_FX1E(opcode);
                } // Add value in VX to I
                0x29 => {
                    self._opcode_FX29(opcode);
                } // Set I to memory of sprite stored in VX
                0x33 => {
                    self._opcode_FX33(opcode);
                } // Store V0-VX inclusive in memory starting at I
                0x65 => {} // Fill V0-VX inclusive with memory starting at I
                _ => panic!("Unknown opcode"),
            },

            _ => panic!("Unknown opcode"),
        }
    }

    // Clear the screen
    #[inline]
    fn _opcode_00E0(&mut self) {
        self.graphics = [0; 64 * 32];
    }

    // Return from subroutine
    #[inline]
    fn _opcode_00EE(&mut self) {
        self.pc = self.stack_pop();
    }

    // Execute machine language subroutine at address NNN
    #[inline]
    fn _opcode_0NNN(&mut self, opcode: u16) {
        println!("Warning: 0NNN opcode called at {:04X}", self.pc);
    }

    // Jump to address NNN
    #[inline]
    fn _opcode_1NNN(&mut self, opcode: u16) {
        self.pc = opcode & 0x0FFF;
    }

    // Execute subroutine starting at address NNN
    #[inline]
    fn _opcode_2NNN(&mut self, opcode: u16) {
        self.stack_push(self.pc);
        self.pc = opcode & 0x0FFF;
    }

    // Skip the following instruction if the value of register VX is not equal to NN
    #[inline]
    fn _opcode_3XNN(&mut self, opcode: u16) {
        let register: usize = reg_x!(opcode);
        let value: u8 = (opcode & 0x00FF) as u8;

        if (self.registers[register] == value) {
            self.pc += 2;
        }
    }

    // Skip the following instruction if the value of register VX is not equal to the value of
    // register VY
    #[inline]
    fn _opcode_4XNN(&mut self, opcode: u16) {
        let register: usize = reg_x!(opcode);
        let value: u8 = (opcode & 0x00FF) as u8;

        if (self.registers[register] != value) {
            self.pc += 2;
        }
    }

    // Skip the following instructionif the value of register VX is equal to the value of register
    // VY
    #[inline]
    fn _opcode_5XY0(&mut self, opcode: u16) {
        let registerX: usize = reg_x!(opcode);
        let registerY: usize = reg_y!(opcode);

        if (self.registers[registerX] == self.registers[registerY]) {
            self.pc += 2;
        }
    }

    // Store number NN in register VX
    #[inline]
    fn _opcode_6XNN(&mut self, opcode: u16) {
        let value: u8 = (opcode & 0x00FF) as u8;
        let register: usize = reg_x!(opcode);

        self.registers[register] = value;
    }

    // Add the value NN to register VX
    #[inline]
    fn _opcode_7XNN(&mut self, opcode: u16) {
        let value: u8 = (opcode & 0x00FF) as u8;
        let register: usize = reg_x!(opcode);

        self.registers[register] = self.registers[register].wrapping_add(value);
    }

    // Store the value of register VY in register VX
    #[inline]
    fn _opcode_8XY0(&mut self, opcode: u16) {
        let registerX = reg_x!(opcode);
        let registerY = reg_y!(opcode);

        self.registers[registerX] = self.registers[registerY];
    }

    // Set VX to VX OR VY
    #[inline]
    fn _opcode_8XY1(&mut self, opcode: u16) {
        let registerX = reg_x!(opcode);
        let registerY = reg_y!(opcode);

        self.registers[registerX] = self.registers[registerX] | self.registers[registerY];
    }

    // Set VX to VX AND VY
    #[inline]
    fn _opcode_8XY2(&mut self, opcode: u16) {
        let registerX = reg_x!(opcode);
        let registerY = reg_y!(opcode);

        self.registers[registerX] = self.registers[registerX] & self.registers[registerY];
    }

    // Set VX to VX XOR VY
    #[inline]
    fn _opcode_8XY3(&mut self, opcode: u16) {
        let registerX = reg_x!(opcode);
        let registerY = reg_y!(opcode);

        self.registers[registerX] = self.registers[registerX] ^ self.registers[registerY];
    }

    // Add the value of register VY to register VX, set VF to 01 if carry occurs  (otherwise 00)
    #[inline]
    fn _opcode_8XY4(&mut self, opcode: u16) {
        let registerX = reg_x!(opcode);
        let registerY = reg_y!(opcode);

        let (result, carry) = self.registers[registerX].overflowing_add(self.registers[registerY]);

        self.registers[registerX] = result;
        self.registers[REG_VF] = if carry { 1 } else { 0 };
    }

    // Subtract VY from VX, set VF if borrow occurs
    #[inline]
    fn _opcode_8XY5(&mut self, opcode: u16) {
        let registerX = reg_x!(opcode);
        let registerY = reg_y!(opcode);

        let (result, borrow) = self.registers[registerX].overflowing_sub(self.registers[registerY]);

        self.registers[registerX] = result;
        self.registers[REG_VF] = if borrow { 1 } else { 0 };
    }

    // Store VX shifted right on bit in register VX, set VF to LSB prior to shift
    #[inline]
    fn _opcode_8XY6(&mut self, opcode: u16) {
        let registerX = reg_x!(opcode);
        let registerY = reg_y!(opcode);

        self.registers[REG_VF] = extract_bits!(self.registers[registerY], 0, 0x1);
        self.registers[registerX] = self.registers[registerY] >> 1;
    }

    // Set VX to VY - VX, set VF if borrow occurs
    #[inline]
    fn _opcode_8XY7(&mut self, opcode: u16) {
        let registerX = reg_x!(opcode);
        let registerY = reg_y!(opcode);

        let (result, borrow) = self.registers[registerY].overflowing_sub(self.registers[registerX]);

        self.registers[registerX] = result;
        self.registers[REG_VF] = if borrow { 1 } else { 0 };
    }

    // Store VY shifted left one bit in VX, set VF to MSB prior to shift
    #[inline]
    fn _opcode_8XYE(&mut self, opcode: u16) {
        let registerX = reg_x!(opcode);
        let registerY = reg_y!(opcode);

        self.registers[REG_VF] = extract_bits!(self.registers[registerY], 7, 0x1);
        self.registers[registerX] = self.registers[registerY] << 1;
    }

    // Skip the following instruction if VX is NOT equal to VY
    #[inline]
    fn _opcode_9XY0(&mut self, opcode: u16) {
        let registerX = reg_x!(opcode);
        let registerY = reg_y!(opcode);

        if (self.registers[registerX] != self.registers[registerY]) {
            // Skip next opcode
            self.pc += 2;
        }
    }

    // Store the memory address NNN in register I
    #[inline]
    fn _opcode_ANNN(&mut self, opcode: u16) {
        let address = extract_bits!(opcode, 0, 0xFFF);
        self.index = address;
    }

    // Jump to address NNN + V0
    #[inline]
    fn _opcode_BNNN(&mut self, opcode: u16) {
        let mut address = extract_bits!(opcode, 0, 0xFFF);
        let sum = address.checked_add(self.registers[0] as u16);

        match sum {
            Some(s) if s <= MAX_ADDRESS => address = s,
            _ => panic!(
                "SEGFAULT: Trying to access invalid address at {:04X}",
                self.pc
            ),
        }

        self.pc = address;
    }

    // Set VX to a random number with a mask of NN
    #[inline]
    fn _opcode_CXNN(&mut self, opcode: u16) {
        let registerX = reg_x!(opcode);
        let mask = extract_bits!(opcode, 0, 0xFF) as u8;

        let random_number: u8 = self.rng.sample(StandardUniform);
        self.registers[registerX] = random_number & mask;
    }

    // Draw a sprite at postion VX, VY with N bytes of sprite data starting at I
    // Set VF if any pixels are changed to unset
    #[inline]
    fn _opcode_DXYN(&mut self, opcode: u16) {
        let coordX = reg_x!(opcode) % DISPLAY_WIDTH;
        let mut coordY = reg_y!(opcode) % DISPLAY_HEIGHT;
        let num_bytes = extract_bits!(opcode, 0, 0xF);

        let sprites = &self.memory[self.index as usize..(self.index + num_bytes) as usize];
        self.registers[REG_VF] = 0;

        for sprite in sprites {
            // Paint each pixel
            for i in 0..8 {
                let x_option = coordX.checked_add(i);

                // If we reached the border of the monitor, go to the next row (Clipping)
                if x_option.is_none() {
                    break;
                }

                // Safe since we checked before
                let x = x_option.unwrap();

                let pixel = self.graphics[coordY * DISPLAY_WIDTH + x];
                let sprite_pixel = extract_bits!(sprite, (7 - i), 0x1);

                if (pixel == 1) && (sprite_pixel == 1) && (self.registers[REG_VF] == 0) {
                    self.registers[REG_VF] = 1;
                }

                self.graphics[coordY * DISPLAY_WIDTH + x] = pixel ^ sprite_pixel;
            }

            // Safely move to next column (=> clipping if necessary)
            if let Some(y) = coordY.checked_add(1) {
                coordY = y;
            } else {
                break;
            }
        }
    }

    // Skip the following instruction if key, corresponding to hex value in VX is pressed
    #[inline]
    fn _opcode_EX9E(&mut self, opcode: u16) {
        let registerX = reg_x!(opcode);
        let value = self.registers[registerX] as usize;

        if (self.keypad[value] == 1) {
            self.pc += 2;
        }
    }

    // Skip the following instruction if key, corresponding to hex value in VX is NOT pressed
    #[inline]
    fn _opcode_EXA1(&mut self, opcode: u16) {
        let registerX = reg_x!(opcode);
        let value = self.registers[registerX] as usize;

        if (self.keypad[value] == 0) {
            self.pc += 2;
        }
    }

    // Store current value of delay in VX
    #[inline]
    fn _opcode_FX07(&mut self, opcode: u16) {
        let registerX = reg_x!(opcode);
        self.registers[registerX] = self.timer_delay;
    }

    // Wait for a keypress and store the result in register VX
    #[inline]
    fn _opcode_FX0A(&mut self, opcode: u16) {
        if let Some(key_index) = self.keypad.iter().position(|&k| k == 1) {
            let reg = reg_x!(opcode);
            self.registers[reg] = key_index as u8;
            return;
        }

        // Decrement to execute this instruction again next cycle
        // Not very pretty, but everything else would be more complicated...
        // Maybe add a flag in the future?
        self.pc -= 2;
    }

    // Set the sound timer to the value of register VX
    #[inline]
    fn _opcode_FX18(&mut self, opcode: u16) {
        let register = reg_x!(opcode);
        self.timer_sound = self.registers[register];
    }

    // Add the value in VX to register I
    #[inline]
    fn _opcode_FX1E(&mut self, opcode: u16) {
        let register = reg_x!(opcode);
        self.index = self.index.wrapping_add(self.registers[register] as u16);
    }

    // Set I to the memory address of the sprite data corresponding to VX
    #[inline]
    fn _opcode_FX29(&mut self, opcode: u16) {
        let register = reg_x!(opcode);
        let digit = self.registers[register];
        self.index = digit as u16 * SIZE_OF_SPRITE;
    }

    // Store the binary-coded decimal equivalent of the value stored in VX at addresses:
    // I, I + 1 and I + 2
    #[inline]
    fn _opcode_FX33(&mut self, opcode: u16) {
        let register = reg_x!(opcode);
        let value = self.registers[register];

        // Bounds checking for debugging
        // Even though rust would panic anyways, this is nicer for debugging
        if self.index > (MAX_ADDRESS - 2) {
            panic!(
                "Opcode FX33 ({:04X}): Not enough memory left! Index would write out-of-bound.",
                self.pc
            );
        }

        for i in 0..3 {
            // TODO: Finish opcode FX33 implementation
            unimplemented!();
        }
    }

    //TODO: Finish last opcode FX65
    #[inline]
    fn _opcode_FX65(&mut self, opcode: u16) {
        unimplemented!();
    }

    //TODO: Integrate push and pop functions into code
    // Helper function to push things on the stack with bounds-checking
    fn stack_push(&mut self, address: u16) {
        // Check bounds
        if self.sp as usize >= self.stack.len() {
            panic!("Stack overflow");
        }

        self.stack[self.sp as usize] = address;
        self.sp += 1;
    }

    // Helper function to pop things from the stack with bounds-checking
    fn stack_pop(&mut self) -> u16 {
        // Check bounds
        self.sp = self.sp.checked_sub(1).expect("Stack underflow");

        return self.stack[self.sp as usize];
    }
}

// ===========================
// Unit tests
// ===========================

// Opcode tests
#[cfg(test)]
mod opcode_tests {
    use super::*;

    // Macro to shadow prelude with pretty_assertions
    //TODO: Find better solution since due to the memory array, things can get very messy
    macro_rules! assert_eq {
        ($($tt:tt)*) => {
            pretty_assertions::assert_eq!($($tt)*)
        };
    }

    // Helper function to load a single opcode as program into the chip
    fn load_opcode(opcode: u16, chip: &mut Chip8) {
        let program = [opcode];
        chip.init(&program);
    }

    // Manually implementing PartialEq for asserts (excluding random rng)
    impl PartialEq for Chip8 {
        fn eq(&self, other: &Self) -> bool {
            self.registers == other.registers
                && self.pc == other.pc
                && self.index == other.index
                && self.timer_delay == other.timer_delay
                && self.timer_sound == other.timer_sound
                && self.memory == other.memory
                && self.stack == other.stack
                && self.sp == other.sp
                && self.graphics == other.graphics
                && self.keypad == other.keypad
        }
    }

    #[test]
    fn test_0NNN() {
        let mut chip = Chip8::new();
        load_opcode(0x0000, &mut chip);

        let mut expected: Chip8 = chip.clone();
        chip.emulateCycle();

        // Only pc should have changed
        expected.pc += 2;
        assert_eq!(expected, chip);
    }

    #[test]
    fn test_00E0() {
        let mut chip = Chip8::new();
        load_opcode(0x00E0, &mut chip);

        // Set display
        chip.graphics.fill(1);

        // Set expected
        let mut expected = chip.clone();
        expected.pc += 2;
        expected.graphics.fill(0);

        // Run cycle
        chip.emulateCycle();

        // Asserts
        assert_eq!(expected, chip);
    }

    mod test_00EE {
        use super::*;

        #[test]
        fn test_00EE_normal() {
            let mut chip = Chip8::new();
            load_opcode(0x00EE, &mut chip);

            // Prepare setup
            chip.stack[chip.sp as usize] = 0x300;
            chip.sp += 1;

            // Set expected
            let mut expected = chip.clone();
            expected.sp -= 1; // Pop first stack entry
            expected.pc = 0x300; // Jump to return-address

            // Run cycle
            chip.emulateCycle();

            // Assert
            assert_eq!(expected, chip);
        }

        #[test]
        #[should_panic]
        fn test_00EE_underflow() {
            let mut chip = Chip8::new();
            load_opcode(0x00EE, &mut chip);

            // Prepare setup
            chip.sp = 0;

            // Run cycle -> should cause a panic
            chip.emulateCycle();
        }
    }

    #[test]
    fn test_1NNN() {
        let mut chip = Chip8::new();
        load_opcode(0x1300, &mut chip);

        // Prepare setup
        let mut expected = chip.clone();
        expected.pc = 0x300;

        // Run cycle
        chip.emulateCycle();

        // Assert
        assert_eq!(expected, chip);
    }

    mod test_2NNN {
        use super::*;

        #[test]
        fn test_2NNN_normal() {
            let mut chip = Chip8::new();
            load_opcode(0x2300, &mut chip);

            // Prepare setup
            let mut expected = chip.clone();
            expected.pc += 2;
            expected.stack[0] = expected.pc;
            expected.pc = 0x300;
            expected.sp = 1;

            // Run cycle
            chip.emulateCycle();

            // Assert
            assert_eq!(expected, chip);
        }

        #[test]
        #[should_panic]
        fn test_2NNN_overflow() {
            let mut chip = Chip8::new();
            load_opcode(0x2300, &mut chip);

            // Prepare setup
            chip.sp = chip.stack.len() as u16;

            // Run cycle -> should panic
            chip.emulateCycle();
        }
    }

    mod test_3XNN {
        use super::*;

        #[test]
        fn test_3XNN_skip() {
            let mut chip = Chip8::new();
            load_opcode(0x3000, &mut chip);

            // Prepare setup
            let mut expected = chip.clone();
            expected.pc += 4;

            // Run cycle
            chip.emulateCycle();

            // Assert
            assert_eq!(expected, chip);
        }

        #[test]
        fn test_3XNN_noskip() {
            let mut chip = Chip8::new();
            load_opcode(0x3001, &mut chip);

            // Prepare setup
            let mut expected = chip.clone();
            expected.pc += 2;

            // Run cycle
            chip.emulateCycle();

            // Assert
            assert_eq!(expected, chip);
        }
    }

    mod test_4XNN {
        use super::*;

        #[test]
        fn test_4XNN_skip() {
            let mut chip = Chip8::new();
            load_opcode(0x4001, &mut chip);

            // Prepare setup
            let mut expected = chip.clone();
            expected.pc += 4;

            // Run cycle
            chip.emulateCycle();

            // Assert
            assert_eq!(expected, chip);
        }

        #[test]
        fn test_4XNN_noskip() {
            let mut chip = Chip8::new();
            load_opcode(0x4000, &mut chip);

            // Prepare setup
            let mut expected = chip.clone();
            expected.pc += 2;

            // Run cycle
            chip.emulateCycle();

            // Assert
            assert_eq!(expected, chip);
        }
    }

    mod test_5XY0 {
        use super::*;

        #[test]
        fn test_5XY0_skip() {
            let mut chip = Chip8::new();
            load_opcode(0x5010, &mut chip);

            // Prepare setup
            let mut expected = chip.clone();
            expected.pc += 4;

            // Run cycle
            chip.emulateCycle();

            // Assert
            assert_eq!(expected, chip);
        }

        #[test]
        fn test_5XY0_noskip() {
            let mut chip = Chip8::new();
            load_opcode(0x5010, &mut chip);

            // Prepare setup
            chip.registers[1] = 1;

            let mut expected = chip.clone();
            expected.pc += 2;

            // Run cycle
            chip.emulateCycle();

            // Assert
            assert_eq!(expected, chip);
        }
    }

    #[test]
    fn test_6XNN() {
        let mut chip = Chip8::new();
        load_opcode(0x6022, &mut chip);

        // Prepare setup
        let mut expected = chip.clone();
        expected.pc += 2;
        expected.registers[0] = 0x22;

        // Run cycle
        chip.emulateCycle();

        // Assert
        assert_eq!(expected, chip);
    }

    mod test_7XNN {
        use super::*;

        #[test]
        fn test_7XNN_normal() {
            let mut chip = Chip8::new();
            load_opcode(0x7022, &mut chip);

            // Prepare setup
            chip.registers[0] = 1;

            let mut expected = chip.clone();
            expected.pc += 2;
            expected.registers[0] += 0x22;

            // Run cycle
            chip.emulateCycle();

            // Assert
            assert_eq!(expected, chip);
        }

        #[test]
        fn test_7XNN_overflow() {
            let mut chip = Chip8::new();
            load_opcode(0x7001, &mut chip);

            // Prepare setup
            chip.registers[0] = 0xFF;

            let mut expected = chip.clone();
            expected.pc += 2;
            expected.registers[0] = 0;

            // Run cycle
            chip.emulateCycle();

            // Assert
            assert_eq!(expected, chip);
        }
    }

    #[test]
    fn test_8XY0() {
        let mut chip = Chip8::new();
        load_opcode(0x8010, &mut chip);

        // Prepare setup
        chip.registers[1] = 1;

        let mut expected = chip.clone();
        expected.pc += 2;
        expected.registers[0] = expected.registers[1];

        // Run cycle
        chip.emulateCycle();

        // Assert
        assert_eq!(expected, chip);
    }

    #[test]
    fn test_8XY1() {
        let cases = [
            (0x01, 0x00, 0x01),
            (0x00, 0x01, 0x01),
            (0x01, 0x01, 0x01),
            (0x00, 0x00, 0x00),
        ];

        for (vx, vy, res) in cases {
            let mut chip = Chip8::new();
            load_opcode(0x8011, &mut chip);

            // Prepare setup
            chip.registers[0] = vx;
            chip.registers[1] = vy;

            let mut expected = chip.clone();
            expected.pc += 2;
            expected.registers[0] = res;

            // Run cycle
            chip.emulateCycle();

            // Assert
            assert_eq!(expected, chip);
        }
    }

    #[test]
    fn test_8XY2() {
        let cases = [
            (0x01, 0x00, 0x00),
            (0x00, 0x01, 0x00),
            (0x01, 0x01, 0x01),
            (0x00, 0x00, 0x00),
        ];

        for (vx, vy, res) in cases {
            let mut chip = Chip8::new();
            load_opcode(0x8012, &mut chip);

            // Prepare setup
            chip.registers[0] = vx;
            chip.registers[1] = vy;

            let mut expected = chip.clone();
            expected.pc += 2;
            expected.registers[0] = res;

            // Run cycle
            chip.emulateCycle();

            // Assert
            assert_eq!(expected, chip);
        }
    }

    #[test]
    fn test_8XY3() {
        let cases = [
            (0x01, 0x00, 0x01),
            (0x00, 0x01, 0x01),
            (0x01, 0x01, 0x00),
            (0x00, 0x00, 0x00),
        ];

        for (vx, vy, res) in cases {
            let mut chip = Chip8::new();
            load_opcode(0x8013, &mut chip);

            // Prepare setup
            chip.registers[0] = vx;
            chip.registers[1] = vy;

            let mut expected = chip.clone();
            expected.pc += 2;
            expected.registers[0] = res;

            // Run cycle
            chip.emulateCycle();

            // Assert
            assert_eq!(expected, chip);
        }
    }

    #[test]
    fn test_8XY4() {
        let cases = [
            (0x00, 0x00, 0x00, 0x00),
            (0x01, 0x01, 0x02, 0x00),
            (0xFF, 0x01, 0x00, 0x01),
        ];

        for (vx, vy, res, vf) in cases {
            let mut chip = Chip8::new();
            load_opcode(0x8014, &mut chip);

            // Prepare setup
            chip.registers[0] = vx;
            chip.registers[1] = vy;

            let mut expected = chip.clone();
            expected.pc += 2;
            expected.registers[0] = res;
            expected.registers[REG_VF] = vf;

            // Run cycle
            chip.emulateCycle();

            // Assert
            assert_eq!(expected, chip);
        }
    }

    #[test]
    fn test_8XY5() {
        let cases = [
            (0x00, 0x00, 0x00, 0x00),
            (0x01, 0x01, 0x00, 0x00),
            (0x00, 0x01, 0xFF, 0x01),
        ];

        for (vx, vy, res, vf) in cases {
            let mut chip = Chip8::new();
            load_opcode(0x8015, &mut chip);

            // Prepare setup
            chip.registers[0] = vx;
            chip.registers[1] = vy;

            let mut expected = chip.clone();
            expected.pc += 2;
            expected.registers[0] = res;
            expected.registers[REG_VF] = vf;

            // Run cycle
            chip.emulateCycle();

            // Assert
            assert_eq!(expected, chip);
        }
    }

    #[test]
    fn test_8XY6() {
        let cases = [(0b1, 0b0, 0b1), (0b10, 0b01, 0b0)];

        for (vy, res, vf) in cases {
            let mut chip = Chip8::new();
            load_opcode(0x8016, &mut chip);

            // Prepare setup
            chip.registers[1] = vy;

            let mut expected = chip.clone();
            expected.pc += 2;
            expected.registers[0] = res;
            expected.registers[REG_VF] = vf;

            // Run cycle
            chip.emulateCycle();

            // Assert
            assert_eq!(expected, chip);
        }
    }

    #[test]
    fn test_8XY7() {
        let cases = [
            (0x00, 0x00, 0x00, 0x00),
            (0x01, 0x01, 0x00, 0x00),
            (0x01, 0x00, 0xFF, 0x01),
        ];

        for (vx, vy, res, vf) in cases {
            let mut chip = Chip8::new();
            load_opcode(0x8017, &mut chip);

            // Prepare setup
            chip.registers[0] = vx;
            chip.registers[1] = vy;

            let mut expected = chip.clone();
            expected.pc += 2;
            expected.registers[0] = res;
            expected.registers[REG_VF] = vf;

            // Run cycle
            chip.emulateCycle();

            // Assert
            assert_eq!(expected, chip);
        }
    }

    #[test]
    fn test_8XYE() {
        let cases = [(0x0, 0x0, 0x0), (0xFF, 0xFE, 0x1)];

        for (vy, res, vf) in cases {
            let mut chip = Chip8::new();
            load_opcode(0x801E, &mut chip);

            // Prepare setup
            chip.registers[1] = vy;

            let mut expected = chip.clone();
            expected.pc += 2;
            expected.registers[0] = res;
            expected.registers[REG_VF] = vf;

            // Run cycle
            chip.emulateCycle();

            // Assert
            assert_eq!(expected, chip);
        }
    }

    #[test]
    fn test_9XY0() {
        let cases = [(0x0, 0x1, 4), (0x0, 0x0, 2)];

        for (vx, vy, pc) in cases {
            let mut chip = Chip8::new();
            load_opcode(0x9010, &mut chip);

            // Prepare setup
            chip.registers[0] = vx;
            chip.registers[1] = vy;

            let mut expected = chip.clone();
            expected.pc += pc;

            // Run cycle
            chip.emulateCycle();

            // Assert
            assert_eq!(expected, chip);
        }
    }

    #[test]
    fn test_ANNN() {
        let mut chip = Chip8::new();
        load_opcode(0xA111, &mut chip);

        // Prepare setup
        chip.index = 0;

        let mut expected = chip.clone();
        expected.index = 0x111;
        expected.pc += 2;

        // Run cycle
        chip.emulateCycle();

        // Assert
        assert_eq!(expected, chip);
    }

    mod test_BNNN {
        use super::*;

        #[test]
        fn test_BNNN_normal() {
            // [NNN, V0, RESULT]
            let cases = [(1, 0, 1), (1, 1, 2)];

            for (n, v, r) in cases {
                let mut chip = Chip8::new();
                load_opcode((0xB000 | n), &mut chip);

                // Prepare setup
                chip.registers[0] = v;

                let mut expected = chip.clone();
                expected.pc = r;

                // Run cycle
                chip.emulateCycle();

                // Assert
                assert_eq!(expected, chip);
            }
        }

        #[test]
        #[should_panic]
        fn test_BNNN_overflow() {
            let mut chip = Chip8::new();
            load_opcode(0xB000 | u16::MAX, &mut chip);

            // Prepare setup
            chip.registers[0] = 1;

            // Run cycle -> should panic
            chip.emulateCycle();
        }

        #[test]
        #[should_panic]
        fn test_BNNN_invalid_address() {
            let mut chip = Chip8::new();
            load_opcode(0xBFFF, &mut chip);

            // Prepare setup
            chip.registers[0] = 1;

            // Run cycle -> should panic
            chip.emulateCycle();
        }
    }

    #[test]
    fn test_CXNN() {
        // We skip this test for now
    }

    #[test]
    fn test_DXYN() {
        let sprites: [u8; 5] = [0b01010101, 0b10101010, 0b01010101, 0b10101010, 0b01010101];

        // Constructing graphic buffer
        let mut graphic_buffer = [0u8; DISPLAY_WIDTH * DISPLAY_HEIGHT];
        graphic_buffer[0..8].copy_from_slice(&[0, 1, 0, 1, 0, 1, 0, 1]);
        let row1 = DISPLAY_WIDTH;
        graphic_buffer[row1..row1 + 8].copy_from_slice(&[1, 0, 1, 0, 1, 0, 1, 0]);
        let row2 = 2 * DISPLAY_WIDTH;
        graphic_buffer[row2..row2 + 8].copy_from_slice(&[0, 1, 0, 1, 0, 1, 0, 1]);
        let row3 = 3 * DISPLAY_WIDTH;
        graphic_buffer[row3..row3 + 8].copy_from_slice(&[1, 0, 1, 0, 1, 0, 1, 0]);
        let row4 = 4 * DISPLAY_WIDTH;
        graphic_buffer[row4..row4 + 8].copy_from_slice(&[0, 1, 0, 1, 0, 1, 0, 1]);

        // Create new chip instance
        let mut chip = Chip8::new();

        // Load program
        let program = [0xD005, 0xD005];

        // Init chip with program
        chip.init(&program);

        let index = 0x250;
        chip.index = index;

        for (i, &sprite) in sprites.iter().enumerate() {
            chip.memory[index as usize + i] = sprite;
        }

        chip.registers[REG_VF] = 0;
        chip.emulateCycle();

        // Assert correct loading into the graphic buffer
        assert_eq!(graphic_buffer, chip.graphics);
        assert_eq!(0, chip.registers[REG_VF]);

        // Run another cycle to control XOR capability
        chip.emulateCycle();
        assert_eq!([0; DISPLAY_WIDTH * DISPLAY_HEIGHT], chip.graphics);
        assert_eq!(1, chip.registers[REG_VF]);
    }

    mod test_EX9E {
        use super::*;

        #[test]
        fn test_EX9E_skip() {
            for key in 0..(0xF + 1) {
                // Create new chip instance
                let mut chip = Chip8::new();

                // Prepare setup
                load_opcode(0xE09E, &mut chip);
                chip.keypad[key] = 1;
                chip.registers[0] = key as u8;

                let mut expected = chip.clone();
                expected.pc += 4;

                // Run cycle
                chip.emulateCycle();
                assert_eq!(expected, chip);
            }
        }

        #[test]
        fn test_EX9E_noskip() {
            for key in 0..(0xF + 1) {
                // Create new chip instance
                let mut chip = Chip8::new();

                // Prepare setup
                load_opcode(0xE09E, &mut chip);
                chip.keypad[key] = 0;
                chip.registers[0] = key as u8;

                let mut expected = chip.clone();
                expected.pc += 2;

                // Run cycle
                chip.emulateCycle();
                assert_eq!(expected, chip);
            }
        }
    }

    #[test]
    fn test_FX07() {
        // Create new chip instance
        let mut chip = Chip8::new();

        // Prepare setup
        load_opcode(0xF007, &mut chip);
        chip.timer_delay = 42;

        let mut expected = chip.clone();
        expected.pc += 2;
        expected.registers[0] = 42;

        // Run cycle
        chip.emulateCycle();
        assert_eq!(expected, chip);
    }

    #[test]
    fn test_FX0A() {
        // Create new chip instance
        let mut chip = Chip8::new();

        // Prepare setup
        load_opcode(0xF00A, &mut chip);
        let mut expected = chip.clone();

        // Run cycle
        chip.emulateCycle();
        assert_eq!(expected, chip);

        // Press key
        chip.keypad[0] = 1;
        expected.keypad[0] = 1;
        expected.pc += 2;
        expected.registers[0] = 0;

        // Run another cycle
        chip.emulateCycle();
        assert_eq!(expected, chip);
    }
}
