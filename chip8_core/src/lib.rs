use rand::Rng;

pub const SCREEN_WIDTH: usize = 64;
pub const SCREEN_HEIGHT: usize = 32;

const RAM_SIZE: usize = 4096;
const NUM_REGS: usize = 16;
const NUM_KEYS: usize = 16;
const STACK_SIZE: usize = 16;
const FONTSET_SIZE: usize = 80;

const START_ADDR: u16 = 0x200;

const FONTSET: [u8; FONTSET_SIZE] = [
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

pub struct Emulator {
    pc: u16,
    ram: [u8; RAM_SIZE],
    screen: [bool; SCREEN_WIDTH * SCREEN_HEIGHT],
    v_regs: [u8; NUM_REGS],
    i_reg: u16,
    sp: u16,
    stack: [u16; STACK_SIZE],
    keys: [bool; NUM_KEYS],
    dt: u8,
    st: u8,
}

impl Emulator {
    pub fn new() -> Self {
        let mut new_emu = Self {
            pc: START_ADDR,
            ram: [0; RAM_SIZE],
            screen: [false; SCREEN_WIDTH * SCREEN_HEIGHT],
            v_regs: [0; NUM_REGS],
            i_reg: 0,
            sp: 0,
            stack: [0; STACK_SIZE],
            keys: [false; NUM_KEYS],
            dt: 0,
            st: 0,
        };
        new_emu.ram[..FONTSET_SIZE].copy_from_slice(&FONTSET);
        new_emu
    }

    pub fn play_sound(&self) -> bool {
        if self.st == 1 {
            return true;
        }
        false
    }

    pub fn reset(&mut self) {
        self.pc = START_ADDR;
        self.ram = [0; RAM_SIZE];
        self.screen = [false; SCREEN_WIDTH * SCREEN_HEIGHT];
        self.v_regs = [0; NUM_REGS];
        self.i_reg = 0;
        self.sp = 0;
        self.stack = [0; STACK_SIZE];
        self.keys = [false; NUM_KEYS];
        self.dt = 0;
        self.st = 0;
        self.ram[..FONTSET_SIZE].copy_from_slice(&FONTSET);
    }

    pub fn tick(&mut self) {
        // fetch
        let op = self.fetch();
        // decode & execute
        self.execute(op);
    }

    pub fn tick_timers(&mut self) {
        if self.dt > 0 {
            self.dt -= 1;
        }

        if self.st > 0 {
            self.st -= 1;
        }
    }

    pub fn get_display(&self) -> &[bool] {
        &self.screen
    }

    pub fn keypress(&mut self, idx: usize, pressed: bool) {
        self.keys[idx] = pressed;
    }

    pub fn load(&mut self, data: &[u8]) {
        let start = START_ADDR as usize;
        let end = start + data.len();
        self.ram[start..end].copy_from_slice(data);
    }

    fn execute(&mut self, op: u16) {
        let digit1 = (op & 0xF000) >> 12;
        let digit2 = (op & 0x0F00) >> 8;
        let digit3 = (op & 0x00F0) >> 4;
        let digit4 = op & 0x000F;

        match (digit1, digit2, digit3, digit4) {
            // NOP
            (0, 0, 0, 0) => return,
            // CLS
            (0, 0, 0xE, 0) => self.screen = [false; SCREEN_WIDTH * SCREEN_HEIGHT],
            // RET
            (0, 0, 0xE, 0xE) => {
                let ret_addr = self.pop();
                self.pc = ret_addr;
            }
            // JP addr
            (1, _, _, _) => {
                let addr = op & 0xFFF;
                self.pc = addr;
            }
            // CALL addr
            (2, _, _, _) => {
                let addr = op & 0xFFF;
                self.push(self.pc);
                self.pc = addr;
            }
            // SE Vx, byte
            (3, _, _, _) => {
                let x = digit2 as usize;
                let kk = (op & 0xFF) as u8;
                if self.v_regs[x] == kk {
                    self.pc += 2;
                }
            }
            // SNE Vx, byte
            (4, _, _, _) => {
                let x = digit2 as usize;
                let kk = (op & 0xFF) as u8;
                if self.v_regs[x] != kk {
                    self.pc += 2;
                }
            }
            // SE Vx, Vy
            (5, _, _, 0) => {
                let x = digit2 as usize;
                let y = digit3 as usize;
                if self.v_regs[x] == self.v_regs[y] {
                    self.pc += 2;
                }
            }
            // LD Vx, byte
            (6, _, _, _) => {
                let kk = (op & 0xFF) as u8;
                let x = digit2 as usize;
                self.v_regs[x] = kk;
            }
            // ADD Vx, byte
            (7, _, _, _) => {
                let kk = (op & 0xFF) as u8;
                let x = digit2 as usize;
                self.v_regs[x] = self.v_regs[x].wrapping_add(kk);
            }
            // LD Vx, Vy
            (8, _, _, 0) => {
                let x = digit2 as usize;
                let y = digit3 as usize;
                self.v_regs[x] = self.v_regs[y];
            }
            // OR Vx, Vy
            (8, _, _, 1) => {
                let x = digit2 as usize;
                let y = digit3 as usize;
                self.v_regs[x] |= self.v_regs[y];
            }
            // AND Vx, Vy
            (8, _, _, 2) => {
                let x = digit2 as usize;
                let y = digit3 as usize;
                self.v_regs[x] &= self.v_regs[y];
            }
            // XOR Vx, Vy
            (8, _, _, 3) => {
                let x = digit2 as usize;
                let y = digit3 as usize;
                self.v_regs[x] ^= self.v_regs[y];
            }
            // ADD Vx, Vy
            (8, _, _, 4) => {
                let x = digit2 as usize;
                let y = digit3 as usize;

                let (new_vx, carry) = self.v_regs[x].overflowing_add(self.v_regs[y]);
                let new_vf = if carry { 1 } else { 0 };

                self.v_regs[x] = new_vx;
                self.v_regs[0xF] = new_vf;
            }
            // SUB Vx, Vy
            (8, _, _, 5) => {
                let x = digit2 as usize;
                let y = digit3 as usize;

                let (new_vx, borrow) = self.v_regs[x].overflowing_sub(self.v_regs[y]);
                let new_vf = if borrow { 0 } else { 1 };

                self.v_regs[x] = new_vx;
                self.v_regs[0xF] = new_vf;
            }
            // SHR Vx {, Vy}
            (8, _, _, 6) => {
                let x = digit2 as usize;
                self.v_regs[0xF] = self.v_regs[x] & 1;
                self.v_regs[x] >>= 1;
            }
            // SUBN Vx, Vy
            (8, _, _, 7) => {
                let x = digit2 as usize;
                let y = digit3 as usize;

                let (new_vx, borrow) = self.v_regs[y].overflowing_sub(self.v_regs[x]);
                let new_vf = if borrow { 0 } else { 1 };

                self.v_regs[x] = new_vx;
                self.v_regs[0xF] = new_vf;
            }
            // SHL Vx, {, Vy}
            (8, _, _, 0xE) => {
                let x = digit2 as usize;
                self.v_regs[0xF] = (self.v_regs[x] >> 7) & 1;
                self.v_regs[x] <<= 1;
            }
            // SNE Vx, Vy
            (9, _, _, 0) => {
                let x = digit2 as usize;
                let y = digit3 as usize;
                if self.v_regs[x] != self.v_regs[y] {
                    self.pc += 2;
                }
            }
            // LD I, addr
            (0xA, _, _, _) => {
                let addr = op & 0xFFF;
                self.i_reg = addr;
            }
            // JP V0, addr
            (0xB, _, _, _) => {
                let addr = op & 0xFFF;
                self.pc = (self.v_regs[0] as u16) + addr;
            }
            // RND Vx, byte
            (0xC, _, _, _) => {
                let x = digit2 as usize;
                let kk = (op & 0xFF) as u8;

                let rng: u8 = rand::thread_rng().gen();
                self.v_regs[x] = rng & kk;
            }
            // DRW Vx, Vy, nibble
            (0xD, _, _, _) => {
                let x_coord = self.v_regs[digit2 as usize] as u16;
                let y_coord = self.v_regs[digit3 as usize] as u16;
                let num_rows = digit4;

                let mut flipped = false;
                for row in 0..num_rows {
                    let addr = self.i_reg + row;
                    let pixels = self.ram[addr as usize];
                    for column in 0..8 {
                        if (pixels & (0b1000_0000 >> column)) != 0 {
                            let x = (x_coord + column) as usize % SCREEN_WIDTH;
                            let y = (y_coord + row) as usize % SCREEN_HEIGHT;

                            let idx = (SCREEN_WIDTH * y) + x;

                            flipped |= self.screen[idx];
                            self.screen[idx] ^= true;
                        }
                    }
                }
                if flipped {
                    self.v_regs[0xF] = 1;
                } else {
                    self.v_regs[0xF] = 0;
                }
            }
            // SKP Vx
            (0xE, _, 9, 0xE) => {
                let vx = self.v_regs[digit2 as usize];
                let key = self.keys[vx as usize];
                if key {
                    self.pc += 2;
                }
            }
            // SKNP Vx
            (0xE, _, 0xA, 1) => {
                let vx = self.v_regs[digit2 as usize];
                let key = self.keys[vx as usize];
                if !key {
                    self.pc += 2;
                }
            }
            // LD Vx, DT
            (0xF, _, 0, 7) => {
                let x = digit2 as usize;
                self.v_regs[x] = self.dt;
            }
            // LD Vx, K
            (0xF, _, 0, 0xA) => {
                let x = digit2 as usize;
                let mut pressed = false;
                for i in 0..self.keys.len() {
                    if self.keys[i] {
                        self.v_regs[x] = i as u8;
                        pressed = true;
                        break;
                    }
                }
                if !pressed {
                    self.pc -= 2;
                }
            }
            // LD DT, Vx
            (0xF, _, 1, 5) => {
                let x = digit2 as usize;
                self.dt = self.v_regs[x];
            }
            // LD ST, Vx
            (0xF, _, 1, 8) => {
                let x = digit2 as usize;
                self.st = self.v_regs[x];
            }
            // ADD I, Vx
            (0xF, _, 1, 0xE) => {
                let x = digit2 as usize;
                self.i_reg += self.v_regs[x] as u16;
            }
            // LD F, Vx
            (0xF, _, 2, 9) => {
                let x = digit2 as usize;
                let c = self.v_regs[x] as u16;
                self.i_reg = c * 5;
            }
            // LD B, Vx
            (0xF, _, 3, 3) => {
                let x = digit2 as usize;
                let vx = self.v_regs[x] as f32;

                let houndreds = (vx / 100.0).floor() as u8;
                let tens = ((vx / 10.0) % 10.0).floor() as u8;
                let ones = (vx % 10.0) as u8;

                self.ram[self.i_reg as usize] = houndreds;
                self.ram[(self.i_reg + 1) as usize] = tens;
                self.ram[(self.i_reg + 2) as usize] = ones;
            }
            // LD [I], Vx
            (0xF, _, 5, 5) => {
                let x = digit2 as usize;
                let addr = self.i_reg as usize;
                for i in 0..=x {
                    self.ram[addr + i] = self.v_regs[i];
                }
            }
            // LD Vx, [I]
            (0xF, _, 6, 5) => {
                let x = digit2 as usize;
                let addr = self.i_reg as usize;
                for i in 0..=x {
                    self.v_regs[i] = self.ram[addr + i];
                }
            }
            (_, _, _, _) => unimplemented!("Unimplemented opcode: {}", op),
        }
    }

    fn fetch(&mut self) -> u16 {
        let higher_byte = self.ram[self.pc as usize] as u16;
        let lower_byte = self.ram[(self.pc + 1) as usize] as u16;
        let op = (higher_byte << 8) | lower_byte;
        self.pc += 2;
        op
    }

    fn push(&mut self, val: u16) {
        self.stack[self.sp as usize] = val;
        self.sp += 1;
    }
    fn pop(&mut self) -> u16 {
        self.sp -= 1;
        self.stack[self.sp as usize]
    }
}
