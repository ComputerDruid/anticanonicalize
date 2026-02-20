use std::{
    io::IoSliceMut,
    mem::MaybeUninit,
    os::unix::process::CommandExt,
    process::{Command, Stdio},
};

use rustix::{
    fs::Dir,
    net::{
        AddressFamily, RecvAncillaryBuffer, RecvFlags, SocketFlags, SocketType, recvmsg, socketpair,
    },
};
use tempfile::TempDir;

fn sendcwd() {
    use std::{
        io::IoSlice,
        mem::MaybeUninit,
        os::fd::AsFd,
        process::{Command, Stdio},
    };

    use rustix::{
        fs::{Mode, OFlags, open},
        io::fcntl_dupfd_cloexec,
        net::{SendAncillaryBuffer, SendFlags, sendmsg},
        stdio::dup2_stdout,
    };

    let tmp = TempDir::with_prefix("anticanonicalize-")
        .expect("creating tempdir")
        .keep();
    Command::new("mount")
        .args(["--bind", "."])
        .arg(&tmp)
        .stderr(Stdio::inherit())
        .output()
        .unwrap();
    let f = open(tmp, OFlags::PATH, Mode::empty()).unwrap();
    let outfd =
        fcntl_dupfd_cloexec(std::io::stdout(), 3).expect("getting new fd for output socket");
    dup2_stdout(std::io::stderr()).unwrap();

    let mut space = [MaybeUninit::uninit(); rustix::cmsg_space!(ScmRights(1))];
    let to_send = [f.as_fd()];
    let mut cmsg_buffer = SendAncillaryBuffer::new(&mut space);
    cmsg_buffer.push(rustix::net::SendAncillaryMessage::ScmRights(&to_send));
    sendmsg(
        outfd,
        &[IoSlice::new(b"DIRFD")],
        &mut cmsg_buffer,
        SendFlags::empty(),
    )
    .unwrap();
}

fn main() {
    if std::env::var("_PLEASE_SEND_YOUR_CWD_TO_STDOUT").is_ok() {
        return sendcwd();
    }
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
    let my_exe = std::path::PathBuf::from("/proc/self/exe")
        .canonicalize()
        .expect("/proc/self/exe should exist");
    let status = Command::new("unshare")
        .args(["--map-root", "--user", "--mount"])
        .arg(my_exe)
        .env("_PLEASE_SEND_YOUR_CWD_TO_STDOUT", "1")
        .stdout(write_sock)
        .stderr(Stdio::inherit())
        .output()
        .unwrap()
        .status;
    assert!(status.success(), "error in unshare/sendcwd: {status:?}");
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
