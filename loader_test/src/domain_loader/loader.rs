use alloc::{
    string::{String, ToString},
    sync::Arc,
    vec,
    vec::Vec,
};
use core::{
    fmt::{Debug, Formatter},
    ops::Range,
};

use linux_kernel_module::{code, kalloc::vm::ModuleArea, println, KernelResult};
use memory_addr::VirtAddr;
use xmas_elf::{program::Type, sections::SectionData, ElfFile};

use crate::mm::{MappingFlags, PAGE_SIZE};

pub struct DomainLoader {
    entry_point: usize,
    data: Arc<Vec<u8>>,
    virt_start: usize,
    ident: String,
    module_area: Option<ModuleArea>,
    text_section: Range<usize>,
}

impl Debug for DomainLoader {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("DomainLoader")
            .field("entry", &self.entry_point)
            .field("phy_start", &self.virt_start)
            .field("ident", &self.ident)
            .finish()
    }
}

impl Clone for DomainLoader {
    fn clone(&self) -> Self {
        Self {
            entry_point: 0,
            data: self.data.clone(),
            virt_start: 0,
            ident: self.ident.to_string(),
            module_area: None,
            text_section: self.text_section.clone(),
        }
    }
}

impl DomainLoader {
    pub fn new(data: Arc<Vec<u8>>, ident: &str) -> Self {
        Self {
            entry_point: 0,
            data,
            virt_start: 0,
            ident: ident.to_string(),
            module_area: None,
            text_section: 0..0,
        }
    }
    pub fn empty() -> Self {
        Self::new(Arc::new(vec![]), "empty_loader")
    }

    fn entry_point(&self) -> usize {
        self.entry_point
    }

    pub fn call(&self) {
        type F = fn() -> usize;
        let main = unsafe { core::mem::transmute::<*const (), F>(self.entry_point() as *const ()) };
        let res = main();
        println!("call domain res: {}", res);
    }

    fn load_program(&mut self, elf: &ElfFile) -> KernelResult<()> {
        elf.program_iter()
            .filter(|ph| ph.get_type() == Ok(Type::Load))
            .for_each(|ph| {
                let start_vaddr = ph.virtual_addr() as usize + self.virt_start;
                let end_vaddr = start_vaddr + ph.mem_size() as usize;
                let mut permission = MappingFlags::empty();
                let ph_flags = ph.flags();
                if ph_flags.is_read() {
                    permission |= MappingFlags::READ;
                }
                if ph_flags.is_write() {
                    permission |= MappingFlags::WRITE;
                }
                if ph_flags.is_execute() {
                    permission |= MappingFlags::EXECUTE;
                }
                let vaddr = VirtAddr::from(start_vaddr).align_down(PAGE_SIZE).as_usize();
                let end_vaddr = VirtAddr::from(end_vaddr).align_up(PAGE_SIZE).as_usize();
                info!(
                    "map range: [{:#x}-{:#x}], memsize:{}, perm:{:?}",
                    vaddr,
                    end_vaddr,
                    ph.mem_size(),
                    permission
                );
                let data =
                    &elf.input[ph.offset() as usize..(ph.offset() + ph.file_size()) as usize];
                let data_len = data.len();
                // direct copy data to kernel space
                let module_area = self.module_area.as_ref().unwrap();
                let module_slice = module_area.as_mut_slice();

                let copy_start = start_vaddr - self.virt_start;
                module_slice[copy_start..copy_start + data_len].copy_from_slice(data);
                info!(
                    "copy data to {:#x}-{:#x}",
                    copy_start,
                    copy_start + data_len
                );
                if permission.contains(MappingFlags::EXECUTE) {
                    self.text_section = vaddr..end_vaddr;
                }
            });
        Ok(())
    }

    fn relocate_dyn(&self, elf: &ElfFile) -> KernelResult<()> {
        if let Ok(res) = relocate_dyn(elf, self.virt_start) {
            warn!("Relocate_dyn {} entries", res.len());
            res.into_iter().for_each(|kv| {
                info!("relocate: {:#x} -> {:#x}", kv.0, kv.1);
                let addr = kv.0;
                unsafe { (addr as *mut usize).write(kv.1) }
            });
            warn!("Relocate_dyn done");
        }
        Ok(())
    }

    pub fn load(&mut self) -> KernelResult<()> {
        let data = self.data.clone();
        let elf_binary = data.as_slice();
        const ELF_MAGIC: [u8; 4] = [0x7f, b'E', b'L', b'F'];
        if elf_binary[0..4] != ELF_MAGIC {
            panic!("Invalid ELF file");
        }
        debug!("Domain address:{:p}", elf_binary.as_ptr());
        let elf = ElfFile::new(elf_binary).map_err(|_| code::EINVAL)?;
        info!("Domain type:{:?}", elf.header.pt2.type_().as_type());
        let end_paddr = elf
            .program_iter()
            .filter(|ph| ph.get_type() == Ok(Type::Load))
            .last()
            .map(|x| x.virtual_addr() as usize + x.mem_size() as usize)
            .unwrap();
        let end_paddr = VirtAddr::from(end_paddr).align_up(PAGE_SIZE);
        // alloc free page to map elf
        let module_area = crate::mm::alloc_free_region(end_paddr.as_usize())?;
        let region_start = module_area.as_ptr() as usize;
        info!(
            "region range:{:#x}-{:#x}",
            region_start,
            region_start + end_paddr.as_usize()
        );
        self.virt_start = region_start;
        self.module_area = Some(module_area);
        self.load_program(&elf)?;
        self.relocate_dyn(&elf)?;

        //
        let text_pages = (self.text_section.end - self.text_section.start) / PAGE_SIZE;
        linux_kernel_module::mm::set_memory_x(self.text_section.start, text_pages).unwrap();
        info!(
            "set_memory_x range: {:#x}-{:#x}",
            self.text_section.start, self.text_section.end
        );

        let entry = elf.header.pt2.entry_point() as usize + region_start;
        info!("entry: {:#x}", entry);
        self.entry_point = entry;
        Ok(())
    }
}

impl Drop for DomainLoader {
    fn drop(&mut self) {
        println!("drop domain loader [{}]", self.ident);
        if let Some(module_area) = self.module_area.take() {
            drop(module_area);
        }
    }
}

fn relocate_dyn(elf: &ElfFile, region_start: usize) -> Result<Vec<(usize, usize)>, &'static str> {
    let data = elf
        .find_section_by_name(".rela.dyn")
        .map(|h| h.get_data(elf).unwrap())
        .ok_or("corrupted .rela.dyn")?;
    let entries = match data {
        SectionData::Rela64(entries) => entries,
        _ => return Err("bad .rela.dyn"),
    };
    let mut res = vec![];
    for entry in entries.iter() {
        match entry.get_type() {
            R_X86_64_RELATIVE => {
                let value = region_start + entry.get_addend() as usize;
                let addr = region_start + entry.get_offset() as usize;
                res.push((addr, value))
            }
            t => unimplemented!("unknown type: {}", t),
        }
    }
    Ok(res)
}

/// Adjust by program base
pub const R_X86_64_RELATIVE: u32 = 8;
