use std::net::UdpSocket;

use juggler::Object;

fn main() -> std::io::Result<()> {
    let socket = UdpSocket::bind("127.0.0.1:34254")?;
    let mut i = 0;
    let mut objs = vec![Object { pos: [32., 21.] }];
    loop {
        // Receives a single datagram message on the socket. If `buf` is too small to hold
        // the message, it will be cut off.
        let mut buf = [0; std::mem::size_of::<Object>()];
        let (amt, src) = socket.recv_from(&mut buf)?;

        unsafe {
            objs[0] = std::mem::transmute(buf);
        }

        println!("[{i}]: Received from {src:?}: {:?}!", objs[0]);

        std::thread::sleep(std::time::Duration::from_millis(10));

        // Redeclare `buf` as slice of the received data and send reverse data back to origin.
        let buf = &mut buf[..amt];
        buf.reverse();
        let amt = socket.send_to(buf, &src)?;
        println!("[{i}]: Sent {amt} bytes!");
        i += 1;
    }
    // Ok(())
}
