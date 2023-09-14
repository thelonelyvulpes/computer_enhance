use sim86_shared::*;

const REG_LEN: usize = 8;
const BIU_LEN: usize = 5;
const MEM_LEN: usize = 10_000;

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

    pub fn execute_instruction(&mut self, inst: &instruction) {
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
            _ => {
                unimplemented!();
            }
        };
        println!(
            "{:0>4X?}{:0>4X?}[{:0>4X}]",
            self.registers.arr, self.registers.biu, self.registers.flags
        );
    }

    unsafe fn execute_mov(&mut self, inst: &instruction) {
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
            }
            _ => {
                panic!("No legal destination for a mov.")
            }
        }
    }

    unsafe fn execute_arithmetic(&mut self, inst: &instruction) {
        let src_inst = inst.Operands[1];
        let dst_inst = inst.Operands[0];
        let dst = self.u16_ptr(dst_inst);
        let mut alu: u32;

        match src_inst.Type {
            operand_type_Operand_Register => {
                let val = *self.register_ptr(src_inst.__bindgen_anon_1.Register.Index as usize);
                match inst.Op {
                    operation_type_Op_add => {
                        alu = (*dst as u32) << 8;
                        alu = alu + ((val as u32) << 8);
                        *dst = ((alu << 8) >> 16) as u16;
                    }
                    operation_type_Op_cmp => {
                        alu = (*dst as u32) << 8;
                        alu = alu - ((val as u32) << 8);
                    }
                    operation_type_Op_sub => {
                        alu = (*dst as u32) << 8;
                        alu = alu - ((val as u32) << 8);
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
                    operation_type_Op_add => {
                        alu = (*dst as u32) << 8;
                        alu = alu + ((val as u32) << 8);
                        *dst = ((alu << 8) >> 16) as u16;
                    }
                    operation_type_Op_cmp => {
                        alu = (*dst as u32) << 8;
                        alu = alu - ((val as u32) << 8);
                    }
                    operation_type_Op_sub => {
                        alu = (*dst as u32) << 8;
                        alu = alu - ((val as u32) << 8);
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

        self.set_zero_flag(alu);
        self.set_signed_flag(alu);
        self.set_overflow_flag(alu);
        self.set_parity_flag(alu);
    }

    unsafe fn execute_u8_arithmetic(&mut self, inst: &instruction) {
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

    unsafe fn u16_ptr(&mut self, inst: instruction_operand) -> *mut u16 {
        match inst.Type {
            operand_type_Operand_Register => {
                let reg_index = inst.__bindgen_anon_1.Register.Index as usize;
                self.register_ptr(reg_index)
            }
            _ => {
                panic!("No legal destination for a mov.")
            }
        }
    }

    fn set_zero_flag(&mut self, result_val: u32) {
        self.registers.flags = if result_val == 0 {
            self.registers.flags | 0x0040u16
        } else {
            self.registers.flags & 0xFFBFu16
        };
    }

    fn set_signed_flag_u8(&mut self, alu: u32) {
        self.registers.flags = if (((alu >> 8) as u8) & 0x80) >> 7 == 1 {
            self.registers.flags | 0x0080u16
        } else {
            self.registers.flags & 0xFF7Fu16
        };
    }

    fn set_signed_flag(&mut self, alu: u32) {
        self.registers.flags = if (((alu >> 8) as u16) & 0x8000) >> 15 == 1 {
            self.registers.flags | 0x0080u16
        } else {
            self.registers.flags & 0xFF7Fu16
        };
    }

    fn set_parity_flag(&mut self, alu: u32) {
        let total = ((alu >> 8) >> 8).count_ones();
        // let mut ctr = alu >> 8;
        // for _ in 0..8 {
        //     total += ctr & 1;
        //     ctr >>= 1;
        // }
        self.registers.flags = if total % 2 == 0 {
            self.registers.flags | 0x0004u16
        } else {
            self.registers.flags & 0xFFFBu16
        }
    }

    fn set_overflow_flag(&mut self, alu: u32) {
        self.registers.flags = if (alu >> 8) > u16::MAX as u32 {
            self.registers.flags | 0x0800u16
        } else {
            self.registers.flags & 0xF7FFu16
        };
    }
    fn set_overflow_flag_u8(&mut self, alu: u32) {
        self.registers.flags = if (alu >> 8) > u8::MAX as u32 {
            self.registers.flags | 0x0800u16
        } else {
            self.registers.flags & 0xF7FFu16
        };
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
