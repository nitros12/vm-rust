#![feature(test)]
#![feature(box_syntax, box_patterns)]

#[macro_use]
extern crate bitflags;
#[macro_use]
extern crate num_derive;
extern crate byteorder;
extern crate num;
extern crate clap;

extern crate test;

use std::env;
use clap::{App, Arg, SubCommand};

mod cpu;
mod memory;
mod instruction;

use ::cpu::Cpu;
use ::instruction::InstrType;

fn main() {
    fn is_int<T: std::str::FromStr>(val: String) -> Result<(), String> {
        if val.parse::<T>().is_ok() {
            Ok(())
        } else {
            Err(String::from("Value must be an integer"))
        }
    }

    let matches = App::new("rustvm")
        .version("0.1.0")
        .author("Ben Simms")
        .about("Virtual machine for compiler")
        .arg(Arg::with_name("input")
             .short("i")
             .long("input")
             .value_name("FILE")
             .help("Program to interpret")
             .required(true)
             .takes_value(true)
             .index(1))
        .arg(Arg::with_name("mem_size")
             .short("m")
             .long("mem_size")
             .takes_value(true)
             .validator(is_int::<usize>)
             .default_value("16")
             .help("Set the memory size, resultant size is 1 << mem_size"))
        .arg(Arg::with_name("num_regs")
             .short("n")
             .long("num_regs")
             .takes_value(true)
             .validator(is_int::<usize>)
             .default_value("10")
             .help("Set the number of registers to use"))
        .subcommand(SubCommand::with_name("test")
                    .about("Enables features for testing correctness of compiler")
                    .arg(Arg::with_name("read_index")
                         .short("d")
                         .long("index")
                         .takes_value(true)
                         .validator(is_int::<u16>)
                         .required(true)
                         .help("Index of memory to read from after program exits"))
                    .arg(Arg::with_name("read_size")
                         .short("s")
                         .long("size")
                         .takes_value(true)
                         .possible_values(&["1", "2", "4", "8"])
                         .default_value("2")
                         .help("Size of memory to read from index"))
                    .arg(Arg::with_name("read_value")
                         .short("v")
                         .long("value")
                         .takes_value(true)
                         .validator(is_int::<u64>)
                         .required(true)
                         .help("Value to compare result of read memory with")))
        .get_matches();

    let mem_size = matches.value_of("mem_size").unwrap().parse::<usize>().unwrap();
    let num_regs = matches.value_of("num_regs").unwrap().parse::<usize>().unwrap();

    let mut cpu = Cpu::new(1 << mem_size, num_regs);

    let fname = matches.value_of("FILE").unwrap();

    cpu.load_file(fname);
    cpu.exe_loop();
}


#[cfg(test)]
mod tests {
    use ::cpu::{CpuIndex, CpuIndexable};
    use ::memory::MemReg;
    use super::*;

    #[test]
    fn test_memory_unsigned() {
        let mut cpu = Cpu::new(1024, 0);

        let tests = [
            MemReg::U1( u8::max_value()),
            MemReg::U2(u16::max_value()),
            MemReg::U4(u32::max_value()),
            MemReg::U8(u64::max_value()),
            MemReg::U1(0),
            MemReg::U2(0),
            MemReg::U4(0),
            MemReg::U8(0),
        ];

        for &r in tests.iter() {
            cpu.write_memory(r, 0);
            assert_eq!(cpu.read_memory(r.size(), 0), r);
        }
    }

    #[test]
    fn test_memory_signed() {
        let mut cpu = Cpu::new(1024, 0);

         let tests = [
            MemReg::U1( i8::max_value() as u8),
            MemReg::U2(i16::max_value() as u16),
            MemReg::U4(i32::max_value() as u32),
            MemReg::U8(i64::max_value() as u64),
            MemReg::U1(0),
            MemReg::U2(0),
            MemReg::U4(0),
            MemReg::U8(0),
            MemReg::U1( i8::min_value() as u8),
            MemReg::U2(i16::min_value() as u16),
            MemReg::U4(i32::min_value() as u32),
            MemReg::U8(i64::min_value() as u64),
        ];

        for &r in tests.iter() {
            cpu.write_memory(r, 0);
            let read = cpu.read_memory(r.size(), 0);
            match (r, read) {
                (MemReg::U1(x), MemReg::U1(y)) => assert_eq!(x as i8, y as i8),
                (MemReg::U2(x), MemReg::U2(y)) => assert_eq!(x as i16, y as i16),
                (MemReg::U4(x), MemReg::U4(y)) => assert_eq!(x as i32, y as i32),
                (MemReg::U8(x), MemReg::U8(y)) => assert_eq!(x as i64, y as i64),
                _ => panic!("Failed to match left and right!"),
            }
        }
    }

    #[test]
    fn test_write_mem() {
        let mut cpu = Cpu::new(1024, 0);
        let val = MemReg::U8(100);
        let mem_index = CpuIndex::make_index(0, false, false);

        cpu.write(val, mem_index.index() as u16);
        assert_eq!(cpu.read_memory(val.size(), mem_index.index()), val);
    }


    #[test]
    fn test_read_mem() {
        let mut cpu = Cpu::new(1024, 0);

        let val = MemReg::U8(100);
        let tests = [
            ((false, false), (false, true)),  // write to 0, read from *0
            ((true,  false), (true, false)),  // write to reg 0, read from reg 0
            ((true,   true), (true,  true)),  // write to mem at reg 0, read from mem at reg 0
        ];

        for &((r_w, d_w), (r_r, d_r)) in tests.iter() {
            let writer = CpuIndex::make_index(0, r_w, d_w);
            assert_eq!(writer.register(), r_w);
            assert_eq!(writer.deref(), d_w);

            let reader = CpuIndex::make_index(0, r_r, d_r);
            assert_eq!(reader.register(), r_r);
            assert_eq!(reader.deref(), d_r);

            cpu.write(val, writer);
            assert_eq!(cpu.read(val.size(), reader), val);
        }
    }
}

