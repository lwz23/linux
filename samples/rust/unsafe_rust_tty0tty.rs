// SPDX-License-Identifier: GPL-2.0
//! Tty0tty test

use core::{default::Default, f32::consts::E, mem, ops::DerefMut, option::Option::None, ptr, result};
use kernel::{
    bit,  device, bindings, 
    file::{File, Operations},
    sync::{smutex::Mutex},
    power, miscdev, Module, 
    prelude::*,
};
use kernel::c_str;
use kernel::io_buffer::{IoBufferReader, IoBufferWriter};


const DRIVER_VERSION: &str = "v1.2";
const DRIVER_AUTHOR: &str = "Wenzhaoliao";
const DRIVER_DESC: &str = "tty0tty null modem driver";

// out
const MCR_DTR: u32 = 0x01;
const MCR_RTS: u32 = 0x02;
const MCR_LOOP: u32 = 0x04;

// in
const MSR_CTS: u32= 0x10;
const MSR_CD: u32= 0x20;
const MSR_DSR: u32 = 0x40;
const MSR_RI: u32 = 0x80;


// modem lines
const TIOCM_LE: u32 = 0x001;      // line enable
const TIOCM_DTR: u32 = 0x002;     // data terminal ready
const TIOCM_RTS: u32 = 0x004;     // request to send
const TIOCM_ST: u32 = 0x010;      // secondary transmit
const TIOCM_SR: u32 = 0x020;      // secondary receive
const TIOCM_CTS: u32 = 0x040;     // clear to send
const TIOCM_CAR: u32 = 0x100;     // carrier detect
const TIOCM_CD: u32 = TIOCM_CAR;  // carrier detect (alias)
const TIOCM_RNG: u32 = 0x200;     // ring
const TIOCM_RI: u32= TIOCM_RNG;  // ring (alias)
const TIOCM_DSR: u32 = 0x400;     // data set ready
const TIOCM_OUT1: u32= 0x2000;
const TIOCM_OUT2: u32 = 0x4000;
const TIOCM_LOOP: u32 = 0x8000;

//for tty0tty_ioctl_tiocgserial
const ASYNC_SKIP_TEST: i32 = 0x40;   // Equivalent to (1 << 6)
const ASYNC_AUTO_IRQ: i32 = 0x80;    // Equivalent to (1 << 7)

//error
const ENOMEM: i32 = 12;
const ENODEV: u32 = 19;

//for ioctl
const TIOCGSERIAL: u32 = 0x541E;
const TIOCMIWAIT: u32 = 0x545C;
const TIOCGICOUNT: u32 = 0x545D;

//for termios
const IGNBRK: u32 = 0o0000001;
const BRKINT: u32 = 0o0000002;
const IGNPAR: u32 = 0o0000004;
const PARMRK: u32 = 0o0000010;
const INPCK: u32 = 0o0000020;

fn relevant_iflag(iflag: u32) -> u32 {
    iflag & (IGNBRK | BRKINT | IGNPAR | PARMRK | INPCK)
}

struct TTY0TTYMethods;

struct Tty0tty {
    _tty: *mut bindings::tty_driver,
}

static mut TPORT: *mut bindings::tty_port = core::ptr::null_mut();

struct Tty0ttySerial {
    tty: *mut bindings::tty_struct, 
    open_count: i32, 

    
    msr: u32, 
    mcr: u32, 

    
    serial: bindings::serial_struct, 
    icount: bindings::async_icount,
}

impl Tty0ttySerial {
    fn new() -> Self {
        Tty0ttySerial {
            tty: ptr::null_mut(),
            open_count: 0,
            msr: 0,
            mcr: 0,
            serial: bindings::serial_struct::default(),
            icount: bindings::async_icount::default(),
        }
    }
}


static mut TTY0TTY_TABLE: *mut *mut Tty0ttySerial = ptr::null_mut();

extern "C" fn tty0tty_open(tty: *mut bindings::tty_struct, file: *mut kernel::bindings::file) -> i32 {
    let mut tty0tty: *mut Tty0ttySerial = core::ptr::null_mut();
    let index: i32;
    let mut msr: u32 = 0;
    let mut mcr: u32 = 0;

    unsafe {
        (*tty).driver_data = core::ptr::null_mut();
        index = (*tty).index;
        tty0tty = *TTY0TTY_TABLE.offset(index as isize);//here meet error, null pointer deference
        if tty0tty.is_null() {
            pr_info!("the tty0tty is null");
            let tty0tty_box = match Box::try_new(Tty0ttySerial::new()) {
                Ok(boxed) => boxed,
                Err(_) => {
                    pr_err!("Failed to allocate memory for Tty0ttySerial");
                    return -ENOMEM; // ENOMEM
                }
            };

            tty0tty = Box::into_raw(tty0tty_box);

            
            unsafe {bindings::sema_init(tty0tty,1);}
            
            *TTY0TTY_TABLE.offset(index as isize) = tty0tty;
        }

        (*TPORT.offset(index as isize)).tty = tty;
        (*tty).port = &mut (*TPORT.offset(index as isize));

        if index % 2 == 0 {
            pr_info!("enter index % 2 == 0");
            let table_entry = *TTY0TTY_TABLE.offset(index as isize + 1);
            pr_info!("assign table_entty");
            if !table_entry.is_null() {
                pr_info!("entry pass !null check");
                if (*table_entry).open_count > 0 {
                    pr_info!("open >0 write mcr");
                    (*table_entry).mcr = mcr;
                }
            }
            /*else{pr_info!("table entry is null");}*/
        } else {
            let table_entry = *TTY0TTY_TABLE.offset(index as isize - 1);
            if !table_entry.is_null() {
                if (*table_entry).open_count > 0 {
                    (*table_entry).mcr = mcr;
                }
            }
        }

        if (mcr & MCR_RTS) == MCR_RTS {
            msr |= MSR_CTS;
        }
        
        if (mcr & MCR_DTR) == MCR_DTR {
            msr |= MSR_DSR | MSR_CD;
        }
        
        (*tty0tty).msr = msr;
        (*tty0tty).mcr = 0;
    
        use core::ffi::c_void;
        (*tty).driver_data = tty0tty as *mut c_void;
        (*tty0tty).tty = tty;
    
        (*tty0tty).open_count += 1;
}
    pr_info!("open success");
    0
}


fn do_close(tty0tty: *mut Tty0ttySerial) {
    pr_info!("starting close");
    unsafe {
        if tty0tty.is_null() || (*tty0tty).tty.is_null() {
            return;
        }

        let mut msr = 0;
        let index = (*(*tty0tty).tty).index;
        if index % 2 == 0 {
            let table_entry = *TTY0TTY_TABLE.offset(index as isize + 1);
            if !table_entry.is_null() {
                if (*table_entry).open_count > 0 {
                    (*table_entry).msr = msr;
                }
            }
        } else {
            let table_entry = *TTY0TTY_TABLE.offset(index as isize - 1);
            if !table_entry.is_null() {
                if (*table_entry).open_count > 0 {
                    (*table_entry).msr = msr;
                }
            }
        }
        pr_info!("pass index check");

        if (*tty0tty).open_count == 0{
            return;
        }
        else{
            (*tty0tty).open_count-=1;
        }
    }
}

extern "C" fn tty0tty_close(tty: *mut bindings::tty_struct, file: *mut kernel::bindings::file) {
    unsafe {
        if tty.is_null() || (*tty).driver_data.is_null() {
            pr_info!("tty is null");
            return; // or handle appropriately
        }
        
        let tty0tty = (*tty).driver_data as *mut Tty0ttySerial;
        do_close(tty0tty);
        pr_info!("close success");
    }
}

extern "C" fn tty0tty_write(tty: *mut bindings::tty_struct, buffer: *const u8, count: i32) -> i32{
    let mut retval=0;
    pr_info!("tty0tty write is called");
    unsafe {
        let tty0tty = (*tty ).driver_data as *mut Tty0ttySerial;
    
        let mut ttyx: *mut bindings::tty_struct = core::ptr::null_mut();

        
        if tty0tty.is_null(){
            pr_info!("tty0tty write is null");
            return -ENOMEM;
        }

        if (*tty0tty).open_count==0{
            pr_info!("open count = 0 return");
            return retval;
        }
        pr_info!("open count = {}",(*tty0tty).open_count);
        pr_info!("tty0tty write check is pass");
        let index = (*(*tty0tty).tty).index;
        if index % 2 == 0 {
            let table_entry = *TTY0TTY_TABLE.offset(index as isize + 1);
            if !table_entry.is_null() {
                if (*table_entry).open_count > 0 {
                    let ttyx = unsafe { (*table_entry).tty };
                }
            }
        } else {
            let table_entry = *TTY0TTY_TABLE.offset(index as isize - 1);
            if !table_entry.is_null() {
                if (*table_entry).open_count > 0 {
                    let ttyx = unsafe { (*table_entry).tty };
                }
            }
        }

        if !ttyx.is_null(){
            bindings::tty_insert_flip_string_fixed_flag((*ttyx).port, buffer, 0,count.try_into().unwrap());
		    bindings::tty_flip_buffer_push((*ttyx).port);
            retval=count;
        }
    }

    pr_info!("tty0tty write is success");

    retval
}

extern "C" fn tty0tty_write_room(tty: *mut bindings::tty_struct) -> u32{
    let mut room=0;
    unsafe {
        let tty0tty = (*tty ).driver_data as *mut Tty0ttySerial;
        if tty0tty.is_null(){
            return 1;//here is some problem
        }

        if (*tty0tty).open_count==0{
            return room;
        }
    }
    room=255;
    room
}

extern "C" fn tty0tty_set_termios(tty: *mut bindings::tty_struct,old_termios:* mut bindings::ktermios){
    let mut cflag:u32;
    let mut iflag:u32;
    
        cflag = (unsafe { *tty }).termios.c_cflag;

        unsafe {
        iflag = (*tty).termios.c_iflag;
        if !old_termios.is_null() {
            if (cflag == (*old_termios).c_cflag) && (relevant_iflag(iflag) == relevant_iflag((*old_termios).c_iflag)) {
                return;
            }
        }
    }
    

}

extern "C" fn tty0tty_tiocmget(tty: *mut bindings::tty_struct)->u32{
    let mut result:u32=0;
    unsafe {
        let tty0tty = (*tty).driver_data as *mut Tty0ttySerial;
        let msr:u32=(*tty0tty ).msr;
        let mcr:u32=(*tty0tty ).mcr;
         result = ((mcr & MCR_DTR) != 0).then_some(TIOCM_DTR).unwrap_or(0) |
             ((mcr & MCR_RTS) != 0).then_some(TIOCM_RTS).unwrap_or(0) |
             ((mcr & MCR_LOOP) != 0).then_some(TIOCM_LOOP).unwrap_or(0) |
             ((msr & MSR_CTS) != 0).then_some(TIOCM_CTS).unwrap_or(0) |
             ((msr & MSR_CD) != 0).then_some(TIOCM_CAR).unwrap_or(0) |
             ((msr & MSR_RI) != 0).then_some(TIOCM_RI).unwrap_or(0) |
             ((msr & MSR_DSR) != 0).then_some(TIOCM_DSR).unwrap_or(0);
    }

    result

}

extern "C" fn tty0tty_tiocmset(tty: *mut bindings::tty_struct,set:u32,clear:u32)->u32{
    unsafe {
        let tty0tty = (*tty).driver_data as *mut Tty0ttySerial;
        let mut mcr=(*tty0tty).mcr;
        let mut msr=0;

        let index = (*(*tty0tty).tty).index;

        if index % 2 == 0 {
            let table_entry = *TTY0TTY_TABLE.offset(index as isize + 1);
            if !table_entry.is_null() {
                if (*table_entry).open_count > 0 {
                    msr = (*table_entry).msr ;
                }
            }
        } else {
            let table_entry = *TTY0TTY_TABLE.offset(index as isize - 1);
            if !table_entry.is_null() {
                if (*table_entry).open_count > 0 {
                    msr = (*table_entry).msr ;
                }
            }
        }
        if set & TIOCM_RTS != 0 {
            mcr |= MCR_RTS;
            msr |= MSR_CTS;
        }
        
        if set & TIOCM_DTR != 0 {
            mcr |= MCR_DTR;
            msr |= MSR_DSR;
            msr |= MSR_CD;
        }
        
        if clear & TIOCM_RTS != 0 {
            mcr &= !MCR_RTS;
            msr &= !MSR_CTS;
        }
        
        if clear & TIOCM_DTR != 0 {
            mcr &= !MCR_DTR;
            msr &= !MSR_DSR;
            msr &= !MSR_CD;
        }
        
        (*tty0tty).mcr=mcr;

        if index % 2 == 0 {
            let table_entry = *TTY0TTY_TABLE.offset(index as isize + 1);
            if !table_entry.is_null() {
                if (*table_entry).open_count > 0 {
                    msr = (*table_entry).msr ;
                }
            }
        } else {
            let table_entry = *TTY0TTY_TABLE.offset(index as isize - 1);
            if !table_entry.is_null() {
                if (*table_entry).open_count > 0 {
                    msr = (*table_entry).msr ;
                }
            }
        }
    }
    
    0
}

extern "C" fn tty0tty_ioctl_tiocgserial(tty: *mut bindings::tty_struct,cmd:u32,arg:u64)->i32{
    unsafe {
        let tty0tty = (*tty).driver_data as *mut Tty0ttySerial;
        if cmd==TIOCGSERIAL{
            let mut tmp=bindings::serial_struct::default();

            if arg==0{
                return -14;
            }
    
            tmp.type_ = (*tty0tty).serial.type_;
            tmp.line = (*tty0tty).serial.line;
            tmp.port = (*tty0tty).serial.port;
            tmp.irq = (*tty0tty).serial.irq;
            tmp.flags = ASYNC_SKIP_TEST | ASYNC_AUTO_IRQ;
            tmp.xmit_fifo_size = (*tty0tty).serial.xmit_fifo_size;
            tmp.baud_base = (*tty0tty).serial.baud_base;
            tmp.close_delay = 5 * 250;
            tmp.closing_wait = 30 * 250;
            tmp.custom_divisor = (*tty0tty).serial.custom_divisor;
            tmp.hub6 = (*tty0tty).serial.hub6;
            tmp.io_type = (*tty0tty).serial.io_type;
        

            //TODO:we don't impl if copy to user here
        }
    }
    
    -515
}

extern "C" fn tty0tty_ioctl_tiocmiwait(tty: *mut bindings::tty_struct,cmd:u32,arg:u64)->i32{
    //TODO:miss asyn part in rust
    0
}

extern "C" fn tty0tty_ioctl_tiocgicount(tty: *mut bindings::tty_struct,cmd:u32,arg:u64)->i32{
    unsafe {
        let tty0tty = (*tty).driver_data as *mut Tty0ttySerial;
        
        if cmd==TIOCGSERIAL{
            let mut cnow=bindings::async_icount::default();
            cnow=(*tty0tty).icount;
            let mut icount=bindings::serial_icounter_struct::default();
            
            icount.cts = cnow.cts as i32;
		    icount.dsr = cnow.dsr as i32;
		    icount.rng = cnow.rng as i32;
		    icount.dcd = cnow.dcd as i32;
		    icount.rx = cnow.rx as i32;
		    icount.tx = cnow.tx as i32;
		    icount.frame = cnow.frame as i32;
		    icount.overrun = cnow.overrun as i32;
		    icount.parity = cnow.parity as i32;
		    icount.brk = cnow.brk as i32;
		    icount.buf_overrun = cnow.buf_overrun as i32;

            //TODO:we don't impl if copy to user here
        }
    }
    -515
}

extern "C" fn tty0tty_ioctl(tty: *mut bindings::tty_struct,cmd:u32,arg:u64)->i32{
    match cmd {
        TIOCGSERIAL => tty0tty_ioctl_tiocgserial(tty, cmd, arg),
        TIOCMIWAIT => tty0tty_ioctl_tiocmiwait(tty, cmd, arg),
        TIOCGICOUNT => tty0tty_ioctl_tiocgicount(tty, cmd, arg),
        _ => {
            // Handle default case or unknown command
            // You can return an error or handle it as needed
            -515 // Example return value for an error
        }
    }
}

static mut SERIAL_OPS: bindings::tty_operations = bindings::tty_operations {
    open: Some(tty0tty_open as extern "C" fn(*mut bindings::tty_struct, *mut kernel::bindings::file)->i32),
    close: Some(tty0tty_close as extern "C" fn(*mut bindings::tty_struct, *mut kernel::bindings::file)),
    write: Some(tty0tty_write as extern "C" fn(*mut bindings::tty_struct,*const u8, i32)->i32),
    write_room: Some(tty0tty_write_room as extern "C" fn(*mut bindings::tty_struct)->u32),
    ioctl: None,
    lookup: None,
    install: None,
    remove: None,
    shutdown: None,
    cleanup: None,
    put_char: None,
    flush_chars: None,
    chars_in_buffer: None,
    compat_ioctl: None,
    set_termios:None,
    throttle: None,
    unthrottle: None,
    stop: None,
    start: None,
    hangup: None,
    break_ctl: None,
    flush_buffer: None,
    set_ldisc: None,
    wait_until_sent: None,
    send_xchar: None,
    tiocmget: None,
    tiocmset: None,
    resize: None,
    get_icount: None,
    get_serial: None,
    set_serial: None,
    show_fdinfo: None,
    proc_show: None,
};

static mut TTY0TTY_TTY_DRIVER: *mut bindings::tty_driver = ptr::null_mut();

unsafe impl Sync for Tty0tty {}

impl kernel::Module for Tty0tty {
    fn init(_name: &'static CStr, mut _module: &'static ThisModule) -> Result<Self> {
        
         // 模仿C代码中的打印信息
         pr_info!("--------------------------\n");
         pr_info!("tty0tty in rust init\n");
         pr_info!("--------------------------\n");
 
         // read pairs，cal pair_count
         let lock = _module.kernel_param_lock();
         let pair_count = 2 * pairs.read(&lock);
         let mut tport_vec: Vec<bindings::tty_port> = Vec::try_with_capacity((pair_count*mem::size_of::<bindings::tty_port>() as i32).try_into().unwrap()).unwrap();
        for _ in 0..(2 * pair_count) {
            tport_vec.try_push(unsafe { mem::zeroed() }).unwrap();
        }

        let mut table_vec: Vec<*mut Tty0ttySerial> = Vec::try_with_capacity((pair_count*mem::size_of::<Tty0ttySerial>() as i32).try_into().unwrap()).unwrap();
        for _ in 0..pair_count {
            // 初始化每个指针为null_mut
            table_vec.try_push(ptr::null_mut()).unwrap();
        }
        unsafe {
            TPORT = Box::into_raw(tport_vec.try_into_boxed_slice().unwrap()) as *mut bindings::tty_port;
            TTY0TTY_TABLE = Box::into_raw(table_vec.try_into_boxed_slice().unwrap()) as *mut *mut Tty0ttySerial;
        }
         let module_ptr = _module.as_ptr();

    unsafe {TTY0TTY_TTY_DRIVER =  bindings::__tty_alloc_driver(pair_count as u32, module_ptr, 0)  ;
        if  unsafe {bindings::IS_ERR(TTY0TTY_TTY_DRIVER as *const core::ffi::c_void)}  {
            let err = unsafe { bindings::PTR_ERR(TTY0TTY_TTY_DRIVER as *const core::ffi::c_void) };
            pr_info!("alloc failed: {}\n", err);
            return Err(kernel::Error::from_kernel_errno(err.try_into().unwrap()));
        }
        else {
            pr_info!("alloc successed with capability:{}",pair_count);
        }
            // 初始化tty driver
            (*TTY0TTY_TTY_DRIVER).owner = module_ptr;
            (*TTY0TTY_TTY_DRIVER).driver_name = c_str!("tty0tty").as_ptr() as *const i8;
            (*TTY0TTY_TTY_DRIVER).name = c_str!("tnt").as_ptr() as *const i8;
            (*TTY0TTY_TTY_DRIVER).major = 240;
            (*TTY0TTY_TTY_DRIVER).minor_start = 16;
            (*TTY0TTY_TTY_DRIVER).type_ = bindings::TTY_DRIVER_TYPE_SERIAL as i16;
            (*TTY0TTY_TTY_DRIVER).subtype = bindings::SERIAL_TYPE_NORMAL as i16;
            (*TTY0TTY_TTY_DRIVER).flags = (bindings::TTY_DRIVER_RESET_TERMIOS | bindings::TTY_DRIVER_REAL_RAW) as u64;
            (*TTY0TTY_TTY_DRIVER).init_termios = bindings::tty_std_termios;
            (*TTY0TTY_TTY_DRIVER).init_termios.c_iflag = 0;
            (*TTY0TTY_TTY_DRIVER).init_termios.c_oflag = 0;
            (*TTY0TTY_TTY_DRIVER).init_termios.c_cflag = bindings::B38400 | bindings::CS8 | bindings::CREAD;
            (*TTY0TTY_TTY_DRIVER).init_termios.c_lflag = 0;
            (*TTY0TTY_TTY_DRIVER).init_termios.c_ispeed = 38400;
            (*TTY0TTY_TTY_DRIVER).init_termios.c_ospeed = 38400;
            
            // 直接设置 ops 字段
            (*TTY0TTY_TTY_DRIVER).ops = &SERIAL_OPS;
        for i in 0..pair_count{
            bindings::tty_port_init(TPORT.add(i.try_into().unwrap()));
            bindings::tty_port_link_device(TPORT.add(i.try_into().unwrap()),TTY0TTY_TTY_DRIVER,i.try_into().unwrap());

        }
 
         // 注册tty driver
         let retval = unsafe { bindings::tty_register_driver(TTY0TTY_TTY_DRIVER) };
         if retval != 0 {
             pr_err!("failed to register tty0tty tty driver");
             unsafe {
                 bindings::tty_driver_kref_put(TTY0TTY_TTY_DRIVER);
             }
             return Err(Error::from_kernel_errno(retval));
         }
 
         pr_info!("tty driver registration created\n");
         pr_info!("{} {} \n",DRIVER_DESC,DRIVER_VERSION);

 
         Ok(Tty0tty { _tty: unsafe { TTY0TTY_TTY_DRIVER } })
        
    }


     }
}

impl Drop for Tty0tty  {
    fn drop(&mut self) {
        pr_info!("tty0tty Exit\n");
    }
}

//TODO: We need to impl the exit fn for the module


module! {
    type: Tty0tty,
    name: b"tty_test",
    author: b"Rust for Linux Contributors",
    description: b"Rust tty module sample",
    license: b"GPL",
    params: {
        short: i32 {
            default: 4,
            permissions: 0o644,
            description: b"tty short param",
        },
        pairs: i32 {
            default: 4,
            permissions: 0o644,
            description: b"tty pairs param",
        },
    },
}