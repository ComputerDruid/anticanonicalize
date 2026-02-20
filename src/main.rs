use std::{
    io::IoSliceMut,
    mem::MaybeUninit,
    os::unix::process::CommandExt,
    process::Command,
};

use rustix::{
    fs::Dir,
    net::{
        AddressFamily, RecvAncillaryBuffer, RecvFlags, SocketFlags, SocketType, recvmsg, socketpair,
    },
};

fn main() {
    let program;
    let args;
    {
        let mut iter = std::env::args_os().skip(1);
        program = iter
            .next()
            .expect("fist argument should be the program to run");
        args = iter;
    }
    let (read_sock, write_sock) = socketpair(
        AddressFamily::UNIX,
        SocketType::DGRAM,
        SocketFlags::empty(),
        None,
    )
    .unwrap();
    Command::new("unshare")
        .args([
            "--map-root",
            "--user",
            "--mount",
            "/home/cdruid/src/sendcwd/target/debug/sendcwd",
        ])
        .stdout(write_sock)
        .output()
        .unwrap();
    let mut space = [MaybeUninit::uninit(); rustix::cmsg_space!(ScmRights(1))];
    let mut cmsg_buffer = RecvAncillaryBuffer::new(&mut space);
    let mut buf = [0u8; 256];
    let msg = recvmsg(
        read_sock,
        &mut [IoSliceMut::new(&mut buf)],
        &mut cmsg_buffer,
        RecvFlags::empty(),
    )
    .unwrap();
    let msg_bytes = &buf[..msg.bytes];
    assert_eq!(msg_bytes, b"DIRFD");
    let cmsg = cmsg_buffer.drain().collect::<Vec<_>>();
    let cmsg: [_; 1] = cmsg
        .try_into()
        .unwrap_or_else(|_| panic!("should be 1 ancillary message"));
    let [cmsg] = cmsg;
    let rustix::net::RecvAncillaryMessage::ScmRights(fd_iter) = cmsg else {
        panic!("unexpected ancillary message type");
    };
    let dirfd = fd_iter
        .map(|fd| Dir::new(fd).expect("received fd should be a directory"))
        .collect::<Vec<_>>();
    let dirfd: [_; 1] = dirfd.try_into().expect("should be 1 fd in SCM_RIGHTS cmsg");
    let [dirfd] = dirfd;
    dirfd.chdir().expect("setting working directory");

    Result::<(), _>::Err(Command::new(program).args(args).env_remove("PWD").exec())
        .expect("exec program to run");
}
