use std::net::UdpSocket;

use juggler::{Object, NUM_OBJS};

fn main() -> std::io::Result<()> {
    let socket = UdpSocket::bind("127.0.0.1:34254")?;
    let mut t = 0;
    let mut objs: Vec<_> = (0..NUM_OBJS).map(|_| Object { pos: [0., 0.] }).collect();
    loop {
        let mut buf = [0; std::mem::size_of::<usize>()];
        let (amt1, _src) = socket.recv_from(&mut buf)?;
        let i = usize::from_le_bytes(buf);
        let mut buf = [0; std::mem::size_of::<Object>()];
        let (amt2, src) = socket.recv_from(&mut buf)?;

        let total_amt = amt1 + amt2;

        if i < objs.len() {
            unsafe {
                objs[i] = std::mem::transmute(buf);
            }
            println!(
                "[{t}]: Received {total_amt} bytes from {src:?}: {i} = {:?}!",
                objs[i]
            );
        }

        // std::thread::sleep(std::time::Duration::from_millis(1000));

        // Redeclare `buf` as slice of the received data and send reverse data back to origin.
        // let buf = &mut buf[..amt];
        // buf.reverse();
        // let amt = socket.send_to(buf, &src)?;
        // println!("[{i}]: Sent {amt} bytes!");
        t += 1;
    }
    // Ok(())
}
