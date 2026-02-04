use std::ptr::null;

use rustix::{
    fs::{AtFlags, Dir, Mode, OFlags, open},
    io::dup,
    path::Arg,
};

fn main() {
    let f = dup(std::io::stdin()).expect("cloning stdin");
    let d = Dir::new(f).unwrap();
    let f = open(".", OFlags::PATH, Mode::empty()).unwrap();
    d.chdir().expect("setting working directory");
    let args = std::env::args_os()
        .skip(1)
        .map(|a| a.into_c_str().unwrap())
        .collect::<Vec<_>>();
    let Some(to_exec) = args.first() else {
        panic!("expected program to run")
    };
    let mut args = args
        .iter()
        .map(|a| a.as_ptr() as *const u8)
        .collect::<Vec<_>>();
    args.push(null());

    Result::<(), _>::Err(unsafe {
        rustix::runtime::execveat(f, &to_exec, args.as_ptr(), null(), AtFlags::empty())
    })
    .expect("calling execve");
}
