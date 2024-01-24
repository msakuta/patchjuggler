use std::net::UdpSocket;

use juggler::Object;

fn main() -> std::io::Result<()> {
    let socket = UdpSocket::bind("127.0.0.1:34255")?;
    let mut i = 0;
    let mut objs = vec![Object { pos: [-3.4, 1.5] }];
    loop {
        let addr = "127.0.0.1:34254";

        let mut buf: [u8; std::mem::size_of::<Object>()] = unsafe { std::mem::transmute(objs[0]) };
        let amt = socket.send_to(&mut buf, addr)?;

        println!("[{i}] Sent {amt} bytes!");

        for obj in &mut objs {
            obj.pos[0] += 0.1;
            obj.pos[1] -= 0.1;
        }

        // Redeclare `buf` as slice of the received data and send reverse data back to origin.
        // let buf = &mut buf[..amt];
        // buf.reverse();
        let (amt, addr) = socket.recv_from(&mut buf)?;

        println!(
            "[{i}] Received response of {amt} bytes from {addr:?}! {:?}",
            &buf[..amt]
        );
        i += 1;
    } // the socket is closed here
      // Ok(())
}
