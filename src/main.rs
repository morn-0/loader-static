use libelf_sys::{Elf64_Ehdr, Elf64_Phdr, Elf64_auxv_t, Elf64_auxv_t__bindgen_ty_1};
use std::{
    arch::asm, env, error::Error, ffi::CString, fs::File, mem::ManuallyDrop, os::fd::AsRawFd, ptr,
    slice,
};

fn main() -> Result<(), Box<dyn Error>> {
    let mut args = env::args();
    assert!(args.next().is_some());
    let args = args.collect::<Vec<String>>();
    let vars = env::vars()
        .map(|(k, v)| format!("{k}={v}"))
        .collect::<Vec<String>>();

    let file = File::open(args.get(0).unwrap())?;
    let mmap = unsafe {
        libc::mmap(
            ptr::null_mut(),
            4096,
            libc::PROT_READ,
            libc::MAP_PRIVATE,
            file.as_raw_fd(),
            0,
        )
    };
    assert!(mmap != libc::MAP_FAILED);

    let ehdr = unsafe { ptr::read(mmap as *mut Elf64_Ehdr) };
    assert!(ehdr.e_type == libc::ET_EXEC && ehdr.e_machine == libc::EM_X86_64);

    let ptr = unsafe { mmap.add(ehdr.e_phoff as usize) as *mut Elf64_Phdr };
    let phdrs = unsafe { slice::from_raw_parts(ptr, ehdr.e_phnum as usize) };

    for phdr in phdrs {
        if phdr.p_type != libc::PT_LOAD {
            continue;
        }

        let map_ptr = phdr.p_vaddr & !(phdr.p_align - 1);

        let mut prot = 0;
        if phdr.p_flags & libc::PF_R != 0 {
            prot |= libc::PROT_READ;
        }
        if phdr.p_flags & libc::PF_W != 0 {
            prot |= libc::PROT_WRITE;
        }
        if phdr.p_flags & libc::PF_X != 0 {
            prot |= libc::PROT_EXEC;
        }

        let mut map_len = phdr.p_filesz + (phdr.p_vaddr % phdr.p_align);
        while map_len % phdr.p_align != 0 {
            map_len += 1;
        }
        let offset = (phdr.p_offset & !(phdr.p_align - 1)) as i64;

        let ret = unsafe {
            libc::mmap(
                map_ptr as *mut libc::c_void,
                map_len as usize,
                prot,
                libc::MAP_PRIVATE | libc::MAP_FIXED,
                file.as_raw_fd(),
                offset,
            )
        };
        assert!(ret != libc::MAP_FAILED);

        let extra_len = phdr.p_memsz - phdr.p_filesz;
        if extra_len > 0 {
            let extra_ptr = map_ptr + map_len;

            let ret = unsafe {
                libc::mmap(
                    extra_ptr as *mut libc::c_void,
                    extra_len as usize,
                    prot,
                    libc::MAP_ANONYMOUS | libc::MAP_PRIVATE | libc::MAP_FIXED,
                    -1,
                    0,
                )
            };
            assert!(ret != libc::MAP_FAILED);
        }
    }

    let mut stack = [0u8; 1 << 20];
    let sp = (stack.as_mut_ptr() as usize + stack.len() - 4096) & !(16 - 1);
    let sp = sp as *mut libc::c_void;

    let mut rnd = [0u8; 16];
    let rnd = rnd.as_mut_ptr() as u64;

    unsafe {
        let push_strs = |mut sp, strs| -> Result<*mut libc::c_void, Box<dyn Error>> {
            for str in strs {
                let str = ManuallyDrop::new(CString::new(str)?);
                sp = push(sp, str.as_ptr());
            }

            Ok(sp)
        };

        let mut sp = push(sp, args.len());

        sp = push_strs(sp, args)?;
        sp = push_strs(sp, vars)?;

        sp = push(sp, 0usize);
        sp = push(
            sp,
            Elf64_auxv_t {
                a_type: libc::AT_RANDOM,
                a_un: Elf64_auxv_t__bindgen_ty_1 { a_val: rnd },
            },
        );
        let _ = push(
            sp,
            Elf64_auxv_t {
                a_type: libc::AT_NULL,
                a_un: Elf64_auxv_t__bindgen_ty_1 { a_val: 0 },
            },
        );
    }

    unsafe {
        asm! {
            "mov rdx, 0",
            "mov rsp, {sp}",
            "jmp {entry}",
            sp = in(reg) sp,
            entry = in(reg) ehdr.e_entry,
        }
    }

    unsafe {
        libc::munmap(mmap, 4096);
    }
    Ok(())
}

unsafe fn push<T>(ptr: *mut libc::c_void, val: T) -> *mut libc::c_void {
    (ptr as *mut T).write(val);
    (ptr as *mut T).wrapping_add(1) as *mut libc::c_void
}
