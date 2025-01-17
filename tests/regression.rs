// Axel '0vercl0k' Souchet - March 17 2024
use std::collections::HashSet;
use std::env;
use std::fs::File;
use std::ops::Range;
use std::path::PathBuf;

use kdmp_parser::{AddrTranslationError, Gpa, Gva, KdmpParserError, KernelDumpParser};
use serde::Deserialize;

/// Convert an hexadecimal encoded integer string into a `u64`.
pub fn hex_str(s: &str) -> u64 {
    let prefix = s.strip_prefix("0x");

    u64::from_str_radix(prefix.unwrap_or(s), 16).unwrap()
}

#[derive(Debug, Deserialize)]
struct M {
    name: String,
    start: String,
    end: String,
}

#[derive(Debug)]
struct Module {
    name: String,
    at: Range<Gva>,
}

impl Into<Module> for M {
    fn into(self) -> Module {
        Module {
            name: self.name,
            at: hex_str(&self.start).into()..hex_str(&self.end).into(),
        }
    }
}

struct TestcaseValues<'test> {
    file: PathBuf,
    dump_type: kdmp_parser::DumpType,
    size: u64,
    phys_addr: u64,
    phys_bytes: [u8; 16],
    virt_addr: u64,
    virt_bytes: [u8; 16],
    rax: u64,
    rbx: u64,
    rcx: u64,
    rdx: u64,
    rsi: u64,
    rdi: u64,
    rip: u64,
    rsp: u64,
    rbp: u64,
    r8: u64,
    r9: u64,
    r10: u64,
    r11: u64,
    r12: u64,
    r13: u64,
    r14: u64,
    r15: u64,
    modules: &'test [Module],
}

fn compare_modules(parser: &KernelDumpParser, modules: &[Module]) -> bool {
    eprintln!("{parser:?}");
    let parser_modules = parser.user_modules().chain(parser.kernel_modules());
    let mut seen = HashSet::new();
    for (r, name) in parser_modules {
        if seen.contains(&r.start) {
            eprintln!("already seen {}", r.start);
            return false;
        }

        let found_mod = modules.iter().find(|m| m.at == *r).unwrap();
        seen.insert(r.start);

        let filename = name.rsplit_once('\\').map(|(_, s)| s).unwrap_or(&name);
        if filename.to_lowercase() != found_mod.name.to_lowercase() {
            if found_mod.name == "nt" && filename == "ntoskrnl.exe" {
                continue;
            }

            eprintln!("{name} {found_mod:?}");
            return false;
        }
    }

    seen.len() == modules.len()
}

#[test]
fn regressions() {
    let base_path =
        PathBuf::from(env::var("TESTDATAS").expect("I need the TESTDATAS env var to work"));

    let test_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests");
    let modules_1: Vec<M> =
        serde_json::from_reader(File::open(test_dir.join("modules_1.json")).unwrap()).unwrap();
    let modules_1 = modules_1
        .into_iter()
        .map(|m| m.into())
        .collect::<Vec<Module>>();
    // kd> r
    // rax=0000000000000003 rbx=fffff8050f4e9f70 rcx=0000000000000001
    // rdx=fffff805135684d0 rsi=0000000000000100 rdi=fffff8050f4e9f80
    // rip=fffff805108776a0 rsp=fffff805135684f8 rbp=fffff80513568600
    // r8=0000000000000003  r9=fffff805135684b8 r10=0000000000000000
    // r11=ffffa8848825e000 r12=fffff8050f4e9f80 r13=fffff80510c3c958
    // r14=0000000000000000 r15=0000000000000052
    // iopl=0         nv up ei pl nz na pe nc
    // cs=0010  ss=0018  ds=002b  es=002b  fs=0053  gs=002b efl=00040202
    let bmp = TestcaseValues {
        file: base_path.join("bmp.dmp"),
        dump_type: kdmp_parser::DumpType::Bmp,
        size: 0x54_4b,
        phys_addr: 0x6d_4d_22,
        phys_bytes: [
            0x6d, 0x00, 0x00, 0x00, 0x00, 0x0a, 0x63, 0x88, 0x75, 0x00, 0x00, 0x00, 0x00, 0x0a,
            0x63, 0x98,
        ],
        virt_addr: 0xfffff805_108776a0,
        virt_bytes: [
            0xcc, 0xc3, 0xcc, 0xcc, 0xcc, 0xcc, 0xcc, 0xcc, 0x0f, 0x1f, 0x84, 0x00, 0x00, 0x00,
            0x00, 0x00,
        ],
        rax: 0x00000000_00000003,
        rbx: 0xfffff805_0f4e9f70,
        rcx: 0x00000000_00000001,
        rdx: 0xfffff805_135684d0,
        rsi: 0x00000000_00000100,
        rdi: 0xfffff805_0f4e9f80,
        rip: 0xfffff805_108776a0,
        rsp: 0xfffff805_135684f8,
        rbp: 0xfffff805_13568600,
        r8: 0x00000000_00000003,
        r9: 0xfffff805_135684b8,
        r10: 0x00000000_00000000,
        r11: 0xffffa884_8825e000,
        r12: 0xfffff805_0f4e9f80,
        r13: 0xfffff805_10c3c958,
        r14: 0x00000000_00000000,
        r15: 0x00000000_00000052,
        modules: modules_1.as_slice(),
    };

    let full = TestcaseValues {
        file: base_path.join("full.dmp"),
        dump_type: kdmp_parser::DumpType::Full,
        size: 0x03_fb_e6,
        phys_addr: 0x6d_4d_22,
        phys_bytes: [
            0x6d, 0x00, 0x00, 0x00, 0x00, 0x0a, 0x63, 0x88, 0x75, 0x00, 0x00, 0x00, 0x00, 0x0a,
            0x63, 0x98,
        ],
        virt_addr: 0xfffff805_108776a0,
        virt_bytes: [
            0xcc, 0xc3, 0xcc, 0xcc, 0xcc, 0xcc, 0xcc, 0xcc, 0x0f, 0x1f, 0x84, 0x00, 0x00, 0x00,
            0x00, 0x00,
        ],
        rax: 0x00000000_00000003,
        rbx: 0xfffff805_0f4e9f70,
        rcx: 0x00000000_00000001,
        rdx: 0xfffff805_135684d0,
        rsi: 0x00000000_00000100,
        rdi: 0xfffff805_0f4e9f80,
        rip: 0xfffff805_108776a0,
        rsp: 0xfffff805_135684f8,
        rbp: 0xfffff805_13568600,
        r8: 0x00000000_00000003,
        r9: 0xfffff805_135684b8,
        r10: 0x00000000_00000000,
        r11: 0xffffa884_8825e000,
        r12: 0xfffff805_0f4e9f80,
        r13: 0xfffff805_10c3c958,
        r14: 0x00000000_00000000,
        r15: 0x00000000_00000052,
        modules: &modules_1,
    };

    let modules_2: Vec<M> =
        serde_json::from_reader(File::open(test_dir.join("modules_2.json")).unwrap()).unwrap();
    let modules_2 = modules_2
        .into_iter()
        .map(|m| m.into())
        .collect::<Vec<Module>>();

    let kernel_dump = TestcaseValues {
        file: base_path.join("kerneldump.dmp"),
        dump_type: kdmp_parser::DumpType::KernelMemory,
        size: 0xa0_2e,
        phys_addr: 0x02_58_92_f0,
        phys_bytes: [
            0x10, 0x8c, 0x24, 0x50, 0x0c, 0xc0, 0xff, 0xff, 0xa0, 0x19, 0x38, 0x51, 0x0c, 0xc0,
            0xff, 0xff,
        ],
        virt_addr: 0xfffff803_f2c35470,
        virt_bytes: [
            0xcc, 0xc3, 0xcc, 0xcc, 0xcc, 0xcc, 0xcc, 0xcc, 0x0f, 0x1f, 0x84, 0x00, 0x00, 0x00,
            0x00, 0x00,
        ],
        rax: 0x00000000_00007a01,
        rbx: 0xffffc00c_5191e010,
        rcx: 0x00000000_00000001,
        rdx: 0x00000012_00000000,
        rsi: 0xffffc00c_51907bb0,
        rdi: 0x00000000_00000002,
        rip: 0xfffff803_f2c35470,
        rsp: 0xfffff803_f515ec28,
        rbp: 0x00000000_0c1c9800,
        r8: 0x00000000_000000b0,
        r9: 0xffffc00c_502ff000,
        r10: 0x00000000_00000057,
        r11: 0xfffff803_f3a04500,
        r12: 0xfffff803_f515ee60,
        r13: 0x00000000_00000003,
        r14: 0xfffff803_f1e9a180,
        r15: 0x00000000_0000001f,
        modules: &modules_2,
    };

    let modules_3: Vec<M> =
        serde_json::from_reader(File::open(test_dir.join("modules_3.json")).unwrap()).unwrap();
    let modules_3 = modules_3
        .into_iter()
        .map(|m| m.into())
        .collect::<Vec<Module>>();

    let kernel_user_dump = TestcaseValues {
        file: base_path.join("kerneluserdump.dmp"),
        dump_type: kdmp_parser::DumpType::KernelAndUserMemory,
        size: 0x01_f7_c7,
        phys_addr: 0x02_58_92_f0,
        phys_bytes: [
            0x10, 0x8c, 0x24, 0x50, 0x0c, 0xc0, 0xff, 0xff, 0xa0, 0x19, 0x38, 0x51, 0x0c, 0xc0,
            0xff, 0xff,
        ],
        virt_addr: 0xfffff803_f2c35470,
        virt_bytes: [
            0xcc, 0xc3, 0xcc, 0xcc, 0xcc, 0xcc, 0xcc, 0xcc, 0x0f, 0x1f, 0x84, 0x00, 0x00, 0x00,
            0x00, 0x00,
        ],
        rax: 0x00000000_00007a01,
        rbx: 0xffffc00c_5191e010,
        rcx: 0x00000000_00000001,
        rdx: 0x00000012_00000000,
        rsi: 0xffffc00c_51907bb0,
        rdi: 0x00000000_00000002,
        rip: 0xfffff803_f2c35470,
        rsp: 0xfffff803_f515ec28,
        rbp: 0x00000000_0c1c9800,
        r8: 0x00000000_000000b0,
        r9: 0xffffc00c_502ff000,
        r10: 0x00000000_00000057,
        r11: 0xfffff803_f3a04500,
        r12: 0xfffff803_f515ee60,
        r13: 0x00000000_00000003,
        r14: 0xfffff803_f1e9a180,
        r15: 0x00000000_0000001f,
        modules: &modules_3,
    };

    let complete_dump = TestcaseValues {
        file: base_path.join("completedump.dmp"),
        dump_type: kdmp_parser::DumpType::CompleteMemory,
        size: 0x01_fb_f9,
        phys_addr: 0x02_58_92_f0,
        phys_bytes: [
            0x10, 0x8c, 0x24, 0x50, 0x0c, 0xc0, 0xff, 0xff, 0xa0, 0x19, 0x38, 0x51, 0x0c, 0xc0,
            0xff, 0xff,
        ],
        virt_addr: 0xfffff803_f2c35470,
        virt_bytes: [
            0xcc, 0xc3, 0xcc, 0xcc, 0xcc, 0xcc, 0xcc, 0xcc, 0x0f, 0x1f, 0x84, 0x00, 0x00, 0x00,
            0x00, 0x00,
        ],
        rax: 0x00000000_00007a01,
        rbx: 0xffffc00c_5191e010,
        rcx: 0x00000000_00000001,
        rdx: 0x00000012_00000000,
        rsi: 0xffffc00c_51907bb0,
        rdi: 0x00000000_00000002,
        rip: 0xfffff803_f2c35470,
        rsp: 0xfffff803_f515ec28,
        rbp: 0x00000000_0c1c9800,
        r8: 0x00000000_000000b0,
        r9: 0xffffc00c_502ff000,
        r10: 0x00000000_00000057,
        r11: 0xfffff803_f3a04500,
        r12: 0xfffff803_f515ee60,
        r13: 0x00000000_00000003,
        r14: 0xfffff803_f1e9a180,
        r15: 0x00000000_0000001f,
        modules: &modules_3,
    };

    let tests = [&bmp, &full, &kernel_dump, &kernel_user_dump, &complete_dump];
    for test in tests {
        let parser = KernelDumpParser::new(&test.file).unwrap();
        assert_eq!(parser.dump_type(), test.dump_type);
        assert_eq!(parser.physmem().len(), test.size as usize);
        let mut buffer = [0; 16];
        parser
            .phys_read_exact(Gpa::new(test.phys_addr), &mut buffer)
            .unwrap();
        assert_eq!(buffer, test.phys_bytes);
        parser
            .virt_read_exact(Gva::new(test.virt_addr), &mut buffer)
            .unwrap();
        assert_eq!(buffer, test.virt_bytes);
        let ctx = parser.context_record();
        assert_eq!(ctx.rax, test.rax);
        assert_eq!(ctx.rbx, test.rbx);
        assert_eq!(ctx.rcx, test.rcx);
        assert_eq!(ctx.rdx, test.rdx);
        assert_eq!(ctx.rsi, test.rsi);
        assert_eq!(ctx.rdi, test.rdi);
        assert_eq!(ctx.rip, test.rip);
        assert_eq!(ctx.rsp, test.rsp);
        assert_eq!(ctx.rbp, test.rbp);
        assert_eq!(ctx.r8, test.r8);
        assert_eq!(ctx.r9, test.r9);
        assert_eq!(ctx.r10, test.r10);
        assert_eq!(ctx.r11, test.r11);
        assert_eq!(ctx.r12, test.r12);
        assert_eq!(ctx.r13, test.r13);
        assert_eq!(ctx.r14, test.r14);
        assert_eq!(ctx.r15, test.r15);
        assert!(compare_modules(&parser, test.modules));
    }

    // Example of a transition PTE readable by WinDbg (in kerneluserdump.dmp):
    // ```
    // kd> db 0x1a42ea30240 l10
    // 000001a4`2ea30240  e0 07 a3 2e a4 01 00 00-80 f2 a2 2e a4 01 00 00  ................
    // kd> !pte 0x1a42ea30240
    //                                            VA 000001a42ea30240
    // PXE at FFFFECF67B3D9018    PPE at FFFFECF67B203480    PDE at FFFFECF640690BA8    PTE at FFFFEC80D2175180
    // contains 0A0000000ECC0867  contains 0A00000013341867  contains 0A000000077AF867  contains 00000000166B7880
    // pfn ecc0      ---DA--UWEV  pfn 13341     ---DA--UWEV  pfn 77af      ---DA--UWEV  not valid
    //                                                                               Transition: 166b7
    // Protect: 4 - ReadWrite
    // ```
    let parser = KernelDumpParser::new(&kernel_user_dump.file).unwrap();
    let mut buffer = [0; 16];
    let expected_buffer = [
        0xe0, 0x07, 0xa3, 0x2e, 0xa4, 0x01, 0x00, 0x00, 0x80, 0xf2, 0xa2, 0x2e, 0xa4, 0x01, 0x00,
        0x00,
    ];
    assert!(parser.virt_read(0x1a42ea30240.into(), &mut buffer).is_ok());
    assert_eq!(buffer, expected_buffer);
    // Example of a valid PTE that don't have a physical page backing it (in
    // kerneldump.dmp):
    // ```
    // kd> !pte 0x1a42ea30240
    //     VA 000001a42ea30240
    // PXE at FFFFECF67B3D9018    PPE at FFFFECF67B203480    PDE at FFFFECF640690BA8    PTE at FFFFEC80D2175180
    // contains 0A0000000ECC0867  contains 0A00000013341867  contains 0A000000077AF867  contains 00000000166B7880
    // pfn ecc0      ---DA--UWEV  pfn 13341     ---DA--UWEV  pfn 77af      ---DA--UWEV  not valid
    //                                            Transition: 166b7
    //                                            Protect: 4 - ReadWrite
    // kd> !db 166b7240
    // Physical memory read at 166b7240 failed
    //
    // kd> !pte 0x16e23fa060
    //     VA 00000016e23fa060
    // PXE at FFFFECF67B3D9000    PPE at FFFFECF67B2002D8    PDE at FFFFECF64005B888    PTE at FFFFEC800B711FD0
    // contains 0A00000001FEB867  contains 0A00000019A08867  contains 0A00000019A07867  contains 8000000001BC4867
    // pfn 1feb      ---DA--UWEV  pfn 19a08     ---DA--UWEV  pfn 19a07     ---DA--UWEV  pfn 1bc4      ---DA--UW-V
    // kd> !db 1bc4000
    // Physical memory read at 1bc4000 failed
    // ```
    let parser = KernelDumpParser::new(&kernel_dump.file).unwrap();
    let mut buffer = [0];
    assert!(matches!(
        parser.virt_read(0x1a42ea30240.into(), &mut buffer),
        Err(KdmpParserError::AddrTranslation(
            AddrTranslationError::Phys(gpa)
        )) if gpa == 0x166b7240.into()
    ));

    assert!(matches!(
        parser.virt_read(0x16e23fa060.into(), &mut buffer),
        Err(KdmpParserError::AddrTranslation(
            AddrTranslationError::Phys(gpa)
        )) if gpa == 0x1bc4060.into()
    ));
}
