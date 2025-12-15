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
    graphics: [u8; 64 * 32],
    keypad: [u8; 16],

    // Utils
    rng: ThreadRng,
}

impl Chip8 {
    // System constants (Specifications)
    //TODO: Move constants outside of chip8 (should be used in struct at compile time)
    // Creating a new chip8 instance
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
            graphics: [0; 64 * 32],
            keypad: [0; 16],

            rng: rand::thread_rng(),
        };
    }

    // Init/Reset a chip8
    fn init(&mut self, program: &[u8]) {
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
        for i in 0..program.len() {
            self.memory[i + self.pc as usize] = program[i];
        }
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
        //TODO: Finish opcode DXYN implementation (what da hell is a sprite?)
        unimplemented!();
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
        if let Some(&key) = self.keypad.iter().find(|&&k| k == 1) {
            let register = reg_x!(opcode);
            self.registers[register] = key;
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
    macro_rules! assert_eq {
        ($($tt:tt)*) => {
            pretty_assertions::assert_eq!($($tt)*)
        };
    }

    fn load_opcode(opcode: u16, chip: &mut Chip8) {
        let low = (opcode & 0x00FF) as u8;
        let high = extract_bits!(opcode, 8, 0xFF) as u8;
        let program = [high, low];

        chip.init(&program);
    }

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
}
