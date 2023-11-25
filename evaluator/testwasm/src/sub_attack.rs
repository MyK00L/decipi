#![allow(unused)]
use wasi::*;

// use stdout for dbg!
macro_rules! dbg {
    () => {
        ::std::println!("[{}:{}]", ::std::file!(), ::std::line!())
    };
    ($val:expr $(,)?) => {
        match $val {
            tmp => {
                ::std::println!("[{}:{}] {} = {:#?}",
                    ::std::file!(), ::std::line!(), ::std::stringify!($val), &tmp);
                tmp
            }
        }
    };
    ($($val:expr),+ $(,)?) => {
        ($(::std::dbg!($val)),+,)
    };
}

unsafe fn test_clock(id: Clockid, precision: Timestamp, it: u32) {
    dbg!(id);
    for _ in 0..it {
        dbg!(clock_res_get(id));
        dbg!(clock_time_get(id, precision));
    }
}



fn main() {
    unsafe {
        const BUF_LEN: usize = 17;
        #[allow(invalid_value)]
        let mut buf: [u8; BUF_LEN] = dbg!(std::mem::MaybeUninit::uninit().assume_init());
        let mut iov = Iovec { buf: buf.as_mut_ptr(), buf_len: BUF_LEN };
        let ciov = Ciovec { buf: buf.as_ptr(), buf_len: BUF_LEN };
        if let Ok(ass) = dbg!(args_sizes_get()) {
            let mut ptrs: Vec<*mut u8> = dbg!(vec![std::ptr::null_mut(); ass.0]);
            let sd: *mut u8 = dbg!(std::ptr::null_mut());
            dbg!(args_get(ptrs.as_mut_ptr(),sd));
        }
        test_clock(CLOCKID_PROCESS_CPUTIME_ID, 0, 3);
        test_clock(CLOCKID_MONOTONIC, 123456789, 3);
        test_clock(CLOCKID_PROCESS_CPUTIME_ID, 123456789, 3);
        if let Ok(ass) = dbg!(environ_sizes_get()) {
            let mut ptrs: Vec<*mut u8> = dbg!(vec![std::ptr::null_mut(); ass.0]);
            let sd: *mut u8 = dbg!(std::ptr::null_mut());
            dbg!(environ_get(ptrs.as_mut_ptr(),sd));
        }
        dbg!(wasi_snapshot_preview1::fd_advise(1,1,1,1));
        dbg!(fd_allocate(0,10,10));
        dbg!(fd_datasync(1));
        dbg!(fd_fdstat_get(1));
        dbg!(fd_filestat_get(1));
        dbg!(fd_pread(0,&[iov],0));
        dbg!(buf);
        dbg!(fd_prestat_dir_name(1,buf.as_mut_ptr(),BUF_LEN));
        dbg!(buf);
        match fd_prestat_get(1) {
            Ok(x) => {
                dbg!(x.tag);
                dbg!(x.u.dir.pr_name_len);
            },
            Err(e) => {dbg!(e);}
        }

        
        dbg!(fd_pwrite(1,&[ciov],0));
        dbg!(fd_readdir(0,buf.as_mut_ptr(),BUF_LEN,0));
        dbg!(fd_renumber(2,3));
        dbg!(fd_seek(1,1,WHENCE_CUR));
        dbg!(fd_sync(1));
        dbg!(fd_tell(1));
        dbg!(fd_write(1,&[ciov]));
        dbg!(path_create_directory(2,"./pwned"));
        dbg!(path_filestat_get(2,1,"."));
        dbg!(path_filestat_set_times(2,0,".",123,123,0));
        dbg!(path_link(3,0,".",2,"."));
        dbg!(path_open(2,0,".",0,0,0,0));
        dbg!(path_readlink(2,".",buf.as_mut_ptr(),BUF_LEN));
        dbg!(buf);
        dbg!(path_remove_directory(1,"./pwned"));
        dbg!(path_rename(0,"0",1,"1"));
        dbg!(path_symlink("./pwned",1,"./pwned2"));
        dbg!(path_unlink_file(3,"./pwned"));
        dbg!(random_get(buf.as_mut_ptr(),BUF_LEN));
        dbg!(buf);
        dbg!(sched_yield());

        dbg!(fd_fdstat_set_flags(0,31));
        dbg!(fd_fdstat_set_rights(0,31,0));
        dbg!(fd_filestat_set_size(0,1000));
        dbg!(fd_filestat_set_times(0,123,123,1));
        dbg!(fd_close(3));
        dbg!(fd_read(0,&[iov]));
        dbg!(buf);
        dbg!(sock_accept(0,0));
        dbg!(sock_recv(0,&[iov],0));
        dbg!(sock_send(0,&[ciov],0));
        dbg!(sock_shutdown(0,0));
        dbg!(wasi_snapshot_preview1::proc_raise(52));
        dbg!(proc_exit(42));
    }
}
