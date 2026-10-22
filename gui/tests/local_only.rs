mod common;

use std::net::{IpAddr, SocketAddr, TcpStream, UdpSocket};
use std::time::Duration;

use common::{Gui, gui_exe};

/// Indirizzo IP della macchina sulla rete locale, se esiste.
fn lan_ip() -> Option<IpAddr> {
    let socket = UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("192.0.2.1:9").ok()?;
    let ip = socket.local_addr().ok()?.ip();
    (!ip.is_loopback() && !ip.is_unspecified()).then_some(ip)
}

#[test]
fn raggiungibile_solo_da_localhost() {
    let gui = Gui::start(gui_exe());
    assert_eq!(ureq::get(&gui.url("/")).call().unwrap().status(), 200);

    if let Some(ip) = lan_ip() {
        let remote = TcpStream::connect_timeout(&SocketAddr::new(ip, gui.port), Duration::from_secs(1));
        assert!(remote.is_err(), "la gui risponde anche su {ip}");
    }
}
