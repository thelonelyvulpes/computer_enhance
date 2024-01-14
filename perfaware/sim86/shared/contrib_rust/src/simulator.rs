use sim86_shared::*;

const REG_LEN: usize = 8;
const BIU_LEN: usize = 5;
const MEM_LEN: usize = u16::MAX as usize;

const PARITY_FLAG: u16 = 0x0004u16;
const SIGNED_FLAG: u16 = 0x0080u16;
const ZERO_FLAG: u16 = 0x0040u16;
const OVERFLOW_FLAG: u16 = 0x0800u16;

struct Registers {
    pub(crate) arr: Vec<u16>,
    pub(crate) flags: u16,
    pub(crate) biu: Vec<u16>,
}

#[allow(dead_code)]
pub(crate) struct Simulator {
    registers: Registers,
    memory: Vec<u8>,
}

unsafe fn into_u8_ptr(x: *mut u16, high: bool) -> *mut u8 {
    let ptr: *mut u8 = x.cast();
    if high {
        return ptr;
    }
    return ptr.add(1);
}

#[allow(non_upper_case_globals)]
impl Simulator {
    pub fn new() -> Self {
        Self {
            registers: Registers {
                arr: vec![0u16; REG_LEN],
                flags: 0u16,
                biu: vec![0u16; BIU_LEN],
            },
            memory: vec![0u8; MEM_LEN],
        }
    }

    pub fn execute_instruction(&mut self, inst: &instruction) -> u16 {
        self.registers.biu[4] += inst.Size as u16;
        match inst.Op {
            operation_type_Op_mov => unsafe {
                self.execute_mov(inst);
            },
            operation_type_Op_add => unsafe {
                if (inst.Flags & 8u32) == 8 {
                    self.execute_arithmetic(inst);
                } else {
                    self.execute_u8_arithmetic(inst);
                }
            },
            operation_type_Op_sub => unsafe {
                if (inst.Flags & 8u32) == 8 {
                    self.execute_arithmetic(inst);
                } else {
                    self.execute_u8_arithmetic(inst);
                }
            },
            operation_type_Op_cmp => unsafe {
                if (inst.Flags & 8u32) == 8 {
                    self.execute_arithmetic(inst);
                } else {
                    self.execute_u8_arithmetic(inst);
                }
            },
            operation_type_Op_jne => unsafe {
                self.cnd_jmp(inst, ZERO_FLAG, false);
            },
            operation_type_Op_je => unsafe {
                self.cnd_jmp(inst, ZERO_FLAG, true);
            },
            operation_type_Op_jnp => unsafe {
                self.cnd_jmp(inst, PARITY_FLAG, false);
            },
            operation_type_Op_jp => unsafe {
                self.cnd_jmp(inst, PARITY_FLAG, true);
            },
            operation_type_Op_jnb => unsafe {
                self.cnd_jmp(inst, OVERFLOW_FLAG, false);
            },
            operation_type_Op_jb => unsafe {
                self.cnd_jmp(inst, OVERFLOW_FLAG, true);
            },
            operation_type_Op_loopnz => unsafe {
                self.cx_loop(inst, false);
            },
            operation_type_Op_loopz => unsafe {
                self.cx_loop(inst, true);
            }
            _ => {
                unimplemented!();
            }
        };

        println!(
            "{:0>4X?}{:0>4X?}[{:0>4X}]",
            self.registers.arr, self.registers.biu, self.registers.flags
        );

        return self.registers.biu[4];
    }

    unsafe fn execute_mov(&mut self, inst: &instruction) -> () {
        let src_inst = inst.Operands[1];
        let dst_inst = inst.Operands[0];
        let wide = (inst.Flags & 8u32) == 8;
        let dst = self.u16_ptr(dst_inst);

        match src_inst.Type {
            operand_type_Operand_Register => {
                if wide {
                    let idx = src_inst.__bindgen_anon_1.Register.Index as usize;
                    *dst = *self.register_ptr(idx);
                } else {
                    let idx = src_inst.__bindgen_anon_1.Register.Index as usize;
                    let high_src = src_inst.__bindgen_anon_1.Register.Offset == 0;
                    let high_dst = dst_inst.__bindgen_anon_1.Register.Offset == 0;
                    let val = *into_u8_ptr(self.register_ptr(idx), high_src);
                    // Shadow 16bit pointer.
                    let dst = into_u8_ptr(dst, high_dst);
                    *dst = val;
                }
            }
            operand_type_Operand_Immediate => {
                if wide {
                    *dst = src_inst.__bindgen_anon_1.Immediate.Value as u16;
                } else {
                    let high_dst = dst_inst.__bindgen_anon_1.Register.Offset == 0;
                    let dst = into_u8_ptr(dst, high_dst);
                    *dst = src_inst.__bindgen_anon_1.Immediate.Value as u8;
                }
            },
            operand_type_Operand_Memory => {
                let address = src_inst.__bindgen_anon_1.Address;
                let src = self.memory_ptr(address.Displacement as usize);
                *dst = *src;
            }
            _ => {
                panic!("No legal destination for a mov.")
            }
        }
    }

    unsafe fn execute_arithmetic(&mut self, inst: &instruction) -> () {
        let src_inst = inst.Operands[1];
        let dst_inst = inst.Operands[0];
        let dst = self.u16_ptr(dst_inst);
        let mut alu: i32;
        match src_inst.Type {
            operand_type_Operand_Register => {
                let val = *self.register_ptr(src_inst.__bindgen_anon_1.Register.Index as usize);
                match inst.Op {
                    operation_type_Op_add => unsafe {
                        alu = (*dst as i32) << 8;
                        alu = alu + ((val as i32) << 8);
                        *dst = ((alu << 8) >> 16) as u16;
                    }
                    operation_type_Op_cmp => unsafe {
                        alu = (*dst as i32) << 8;
                        alu = alu - ((val as i32) << 8);
                    }
                    operation_type_Op_sub => unsafe {
                        alu = (*dst as i32) << 8;
                        alu = alu - ((val as i32) << 8);
                        *dst = ((alu << 8) >> 16) as u16;
                    }
                    _ => {
                        panic!("Illegal operation type");
                    }
                };
            }
            operand_type_Operand_Immediate => {
                let val = src_inst.__bindgen_anon_1.Immediate.Value as u16;
                match inst.Op {
                    operation_type_Op_add => unsafe {
                        alu = (*dst as i32) << 8;
                        alu = alu + ((val as i32) << 8);
                        *dst = ((alu << 8) >> 16) as u16;
                    }
                    operation_type_Op_cmp => unsafe {
                        alu = (*dst as i32) << 8;
                        alu = alu - ((val as i32) << 8);
                    }
                    operation_type_Op_sub => unsafe {
                        alu = (*dst as i32) << 8;
                        alu = alu - ((val as i32) << 8);
                        *dst = ((alu << 8) >> 16) as u16;
                    }
                    _ => {
                        panic!("Illegal operation type");
                    }
                };
            }
            _ => {
                panic!("Illegal operand for op.")
            }
        };

        self.set_zero_flag(alu as u32);
        self.set_signed_flag(alu as u32);
        self.set_overflow_flag(alu as u32);
        self.set_parity_flag(alu as u32);
    }

    unsafe fn execute_u8_arithmetic(&mut self, inst: &instruction) -> () {
        let src_inst = inst.Operands[1];
        let dst_inst = inst.Operands[0];
        let dst = self.u16_ptr(dst_inst);
        let mut alu: u32;

        match src_inst.Type {
            operand_type_Operand_Register => {
                let reg_index = src_inst.__bindgen_anon_1.Register.Index as usize;
                let high_src = src_inst.__bindgen_anon_1.Register.Offset == 0;
                let val = *into_u8_ptr(self.register_ptr(reg_index), high_src);
                let high_dst = dst_inst.__bindgen_anon_1.Register.Offset == 0;
                let sub_dst = into_u8_ptr(dst, high_dst);
                match inst.Op {
                    operation_type_Op_add => {
                        alu = (*sub_dst as u32) << 8;
                        alu = alu + ((val as u32) << 8);
                        *sub_dst = ((alu << 16) >> 24) as u8;
                    }
                    operation_type_Op_cmp => {
                        alu = (*sub_dst as u32) << 8;
                        alu = alu - ((val as u32) << 8);
                    }
                    operation_type_Op_sub => {
                        alu = (*sub_dst as u32) << 8;
                        alu = alu - ((val as u32) << 8);
                        *sub_dst = ((alu << 16) >> 24) as u8;
                    }
                    _ => {
                        panic!("Illegal operation type");
                    }
                };
            }
            operand_type_Operand_Immediate => {
                let sub_dst = into_u8_ptr(dst, dst_inst.__bindgen_anon_1.Register.Offset == 0);
                let val = src_inst.__bindgen_anon_1.Immediate.Value as u8;
                match inst.Op {
                    operation_type_Op_add => {
                        alu = (*sub_dst as u32) << 8;
                        alu = alu + ((val as u32) << 8);
                        *sub_dst = ((alu << 16) >> 24) as u8;
                    }
                    operation_type_Op_cmp => {
                        alu = (*sub_dst as u32) << 8;
                        alu = alu - ((val as u32) << 8);
                    }
                    operation_type_Op_sub => {
                        alu = (*sub_dst as u32) << 8;
                        alu = alu - ((val as u32) << 8);
                        *sub_dst = ((alu << 16) >> 24) as u8;
                    }
                    _ => {
                        panic!("Illegal operation type");
                    }
                };
            }
            _ => {
                panic!("Illegal operand for op.")
            }
        };

        self.set_zero_flag(alu);
        self.set_signed_flag_u8(alu);
        self.set_overflow_flag_u8(alu);
        self.set_parity_flag(alu);
    }

    unsafe fn register_ptr(&mut self, idx: usize) -> *mut u16 {
        if idx > REG_LEN {
            if idx > REG_LEN + BIU_LEN {
                panic!("illegal access");
            }
            let biu_index = idx - (REG_LEN + 1);
            return self.registers.biu.as_mut_ptr().add(biu_index);
        }
        return self.registers.arr.as_mut_ptr().add(idx - 1);
    }

    unsafe fn memory_ptr(&mut self, idx: usize) -> *mut u16 {
        if idx > MEM_LEN {
            panic!("illegal access")
        }

        return self.memory[idx-1..idx].align_to_mut::<u16>().1.as_mut_ptr();
    }

    unsafe fn u16_ptr(&mut self, inst: instruction_operand) -> *mut u16 {
        match inst.Type {
            operand_type_Operand_Register => {
                let reg_index = inst.__bindgen_anon_1.Register.Index as usize;
                self.register_ptr(reg_index)
            }
            operand_type_Operand_Memory => {
                let address = inst.__bindgen_anon_1.Address;

                if address.Terms[0].Register.Index == 0 {
                    let mem_index = address.Displacement as usize;
                    return self.memory_ptr(mem_index);
                }
                let idx = self.registers.arr[(address.Terms[0].Register.Index -1) as usize] as usize;
                let idx = idx + address.Displacement as usize;
                return self.memory_ptr(idx);
            }
            _ => {
                panic!("No legal destination for a mov.")
            }
        }
    }

    fn set_zero_flag(&mut self, result_val: u32) -> () {
        self.registers.flags = if result_val == 0 {
            self.registers.flags | ZERO_FLAG
        } else {
            self.registers.flags & (u16::MAX - ZERO_FLAG)
        };
    }

    fn set_signed_flag_u8(&mut self, alu: u32) -> () {
        self.registers.flags = if (((alu >> 8) as u8) & 0x80) >> 7 == 1 {
            self.registers.flags | SIGNED_FLAG
        } else {
            self.registers.flags & (u16::MAX - SIGNED_FLAG)
        };
    }

    fn set_signed_flag(&mut self, alu: u32) -> () {
        self.registers.flags = if (((alu >> 8) as u16) & 0x8000) >> 15 == 1 {
            self.registers.flags | SIGNED_FLAG
        } else {
            self.registers.flags & (u16::MAX - SIGNED_FLAG)
        };
    }

    fn set_parity_flag(&mut self, alu: u32) -> () {
        let total = ((alu >> 8) >> 8).count_ones();
        self.registers.flags = if total % 2 == 0 {
            self.registers.flags | PARITY_FLAG
        } else {
            self.registers.flags & (u16::MAX - PARITY_FLAG)
        }
    }

    fn set_overflow_flag(&mut self, alu: u32) -> () {
        self.registers.flags = if (alu >> 8) > u16::MAX as u32 {
            self.registers.flags | OVERFLOW_FLAG
        } else {
            self.registers.flags & (u16::MAX - OVERFLOW_FLAG)
        };
    }

    fn set_overflow_flag_u8(&mut self, alu: u32) -> () {
        self.registers.flags = if (alu >> 8) > u8::MAX as u32 {
            self.registers.flags | OVERFLOW_FLAG
        } else {
            self.registers.flags & (u16::MAX - OVERFLOW_FLAG)
        };
    }

    unsafe fn cnd_jmp(&mut self, jmp: &instruction, flag: u16, exp: bool) -> () {
        let zero = self.registers.flags & flag > 0;
        if zero == exp {
            self.set_ip_to_jmp(jmp);
        }
    }

    unsafe fn set_ip_to_jmp(&mut self, jmp: &instruction) {
        let current = self.registers.biu[4];
        let abs = jmp.Operands[0].__bindgen_anon_1.Immediate.Value.abs() as u16;
        if jmp.Operands[0].__bindgen_anon_1.Immediate.Value > 0 {
            self.registers.biu[4] = current + abs;
        } else {
            if abs > current {
                panic!("Broken code")
            }
            self.registers.biu[4] = current - abs;
        }
    }

    unsafe fn cx_loop(&mut self, jmp: &instruction, exp: bool) -> () {
        self.registers.arr[2] -= 1;
        if ((self.registers.flags & ZERO_FLAG) > 1) == exp && self.registers.arr[2] != 0 {
            self.set_ip_to_jmp(jmp);
        }
    }

}

#[cfg(test)]
mod tests {
    #[test]
    fn flag_swap_on() {
        let pre = 0xF000u16;
        assert_eq!(0b1111_0000_0010_0000u16, pre | 0x0020u16)
    }

    #[test]
    fn flag_swap_off() {
        let pre = 0b1111_0000_1111_0000u16;
        assert_eq!(0xF0D0u16, pre & 0xFFDFu16)
    }

    #[test]
    fn bit_counting() {
        let mut total = 57u16.count_ones();
        assert_eq!(4, total)
    }

    #[test]
    fn shift() {
        let val = 1;
        let shifted = val >> 1;
        assert_eq!(0, shifted);
    }
}
