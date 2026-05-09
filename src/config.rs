#[derive(Debug, Clone, Copy)]
pub struct Chip8Config {
    pub shift_inplace: bool, // Wether 8XY6, 8XYE should shift inplace VX or copy VY into VX beforehand
    pub BNNN_quirk: bool,    // Wether BNNN or BXNN should be executed for this opcode
}

impl Default for Chip8Config {
    fn default() -> Self {
        Self {
            shift_inplace: false,
            BNNN_quirk: false,
        }
    }
}

impl Chip8Config {
    pub fn modern() -> Self {
        Self {
            shift_inplace: true,
            BNNN_quirk: true,
        }
    }
}
