extern crate crc;

#[cfg(has_spiflash)]
mod imp {
    use core::str;
    use cache;
    use spiflash;
    use spiflash::Error;

    use core::slice;
    use core::cmp;
    use csr;
    use byteorder::{ByteOrder, BigEndian};
    use flash::crc::crc32;

    const SIZE: usize = spiflash::SECTOR_SIZE;

    #[cfg(soc_platform = "kasli")]
    const ADDR: [usize; 3] = [
        0x000000,
        0x400000,
        ::mem::FLASH_BOOT_ADDRESS,
    ];

    #[cfg(any(soc_platform = "sayma_amc", soc_platform = "metlino"))]
    const ADDR: [usize; 3] = [
        0x000000,
        0x000000,
        ::mem::FLASH_BOOT_ADDRESS,
    ];

    #[cfg(soc_platform = "kc705")]
    const ADDR: [usize; 3] = [
        0x000000,
        0xaf0000,
        ::mem::FLASH_BOOT_ADDRESS,
    ];

    use spiflash::lock::Lock;
    // #[cfg(soc_platform = "metlino")]
    // use spiflash_bitstream::lock::Lock;

    // pub fn erase_sector(data: &'static [u8]) {
    //     // let data = lock.data(addr);
    //     println!("get data");
    //     unsafe { spiflash::erase_sector(data.as_ptr() as usize) }; // problematic
    //     println!("Erased");
    //     cache::flush_l2_cache();
    // }

    // // pub fn erase_sector_bitstream(addr: usize) -> Result<(), Error> {
    // //     unsafe { spiflash_bitstream::erase_sector(addr) };
    // //     cache::flush_l2_cache();
    // //     Ok(())
    // // }
    
    // pub fn write(key: &str, mut value: &[u8]) -> Result<(), Error> {
    //     // println!("Writing in flash");
    //     if key == "firmware" {
    //         let expected_len = BigEndian::read_u32(&value[0..4]) + 8;
    //         let actual_crc = crc32::checksum_ieee(&value[8..]);
    //         let expected_crc = BigEndian::read_u32(&value[4..8]);
    //         if expected_crc != actual_crc {
    //             return Err(Error::CorruptedFirmware);
    //         }
    //         if expected_len != value.len() as u32 {
    //             return Err(Error::CorruptedFirmware);
    //         }
    //     }
    //     let addr: usize;
    //     match key {
    //         "gateware" => { addr = ADDR[0]; }
    //         "bootloader" => { addr = ADDR[1]; }
    //         "firmware" => { addr = ADDR[2]; }
    //         _ => { return Err(Error::WrongPartition); }
    //     }
    //     let lock = Lock::take()?;
    //     // println!("the addr is: {}", addr);
    //     let firstsector: usize = addr / SIZE; 
    //     let lastsector: usize = (addr + value.len() - 1) / SIZE;
    //     for offset in firstsector..lastsector+1 {
    //         let size = cmp::min(SIZE as usize, value.len());
    //         if cfg!(any(soc_platform = "sayma_amc", soc_platform = "metlino")) && key == "gateware" {
    //             println!("In metlino");
    //             // erase_sector_bitstream(SIZE * offset)?;
    //             // unsafe { spiflash_bitstream::write(SIZE * offset as usize, &value[..size]) };
    //             // Verifying
    //             // let get = unsafe { slice::from_raw_parts (
    //                 //     (0x02000000 + SIZE * offset) as *const u8, SIZE) };
    //                 // for i in 0..size {
    //                     //     if value[i as usize] != get[i as usize] {
    //                         //         return Err(Error::WriteFail { sector: offset });
    //                         //     }
    //                         // }
    //             // println!("Verifying at offset {} pass", offset)
    //         } else {
    //             println!("NOT in metlino");
    //             let data = lock.data(SIZE * offset);
    //             println!("lock success");
    //             erase_sector(data);
    //             println!("sector {} erased", offset);
    //             unsafe { spiflash::write(SIZE * offset as usize, &value[..size]) };
    //             println!("write success");
    //             cache::flush_l2_cache();
    //             // Verifying
    //             let get = unsafe { slice::from_raw_parts (
    //                     (SIZE * offset) as *const u8, SIZE) };
    //             for i in 0..size {
    //                 if value[i as usize] != get[i as usize] {
    //                     return Err(Error::WriteFail { sector: offset });
    //                 }
    //             }
    //             println!("Verifying at offset {} pass", offset)
    //         }
    //         value = &value[size..];
    //         if value.len() <= 0 {
    //             break;
    //         }
    //     }
    //     Ok(())
    // }

    pub fn erase_sector(lock: &Lock, addr: usize) -> Result<(), Error> {
        let data = lock.data(addr);
        unsafe { spiflash::erase_sector(data.as_ptr() as usize) };
        cache::flush_l2_cache();
        Ok(())
    }
    
    pub fn write(key: &str, mut value: &[u8]) -> Result<(), Error> {
        if key == "firmware" {
            let expected_len = BigEndian::read_u32(&value[0..4]) + 8;
            let actual_crc = crc32::checksum_ieee(&value[8..]);
            let expected_crc = BigEndian::read_u32(&value[4..8]);
            if expected_crc != actual_crc {
                return Err(Error::CorruptedFirmware);
            }
            if expected_len != value.len() as u32 {
                return Err(Error::CorruptedFirmware);
            }
        }
        let addr: usize;
        match key {
            "gateware" => { addr = ADDR[0]; }
            "bootloader" => { addr = ADDR[1]; }
            "firmware" => { addr = ADDR[2]; }
            _ => { return Err(Error::WrongPartition); }
        }
        let mut lock = Lock::take()?;
        let firstsector: usize = addr / SIZE; 
        let lastsector: usize = (addr + value.len() - 1) / SIZE;
        for offset in firstsector..lastsector+1 {
            let size = cmp::min(SIZE as usize, value.len());
            erase_sector(&mut lock, SIZE * offset)?;
            unsafe { spiflash::write(SIZE * offset as usize, &value[..size]) };
            cache::flush_l2_cache();
            // Verifying
            let get = unsafe { slice::from_raw_parts (
                    (SIZE * offset) as *const u8, SIZE) };
            for i in 0..size {
                if value[i as usize] != get[i as usize] {
                    return Err(Error::WriteFail { sector: offset });
                }
            }
            value = &value[size..];
            if value.len() <= 0 {
                break;
            }
        }
        Ok(())
    }

}

#[cfg(not(has_spiflash))]
mod imp {
    use super::Error;

    pub fn erase_sector(addr: usize) -> Result<(), Error> {
        f(Err(Error::NoFlash))
    }

    pub fn write(key: &str, mut value: &[u8]) -> Result<(), Error> {
        Err(Error::NoFlash)
    }

    pub fn reload () -> Result<(), Error> {
        Err(Error::NoFlash)
    }
}

pub use self::imp::*;
