// SPDX-License-Identifier: AGPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 Jareer Abdullah and Concat contributors

//! Pairing a phone with this machine over the local network, with nothing
//! in between: no account, no VPN.
//!
//! The server's JSON-RPC door is plaintext and loopback. This module adds
//! the door a phone can use: a TLS listener whose certificate the machine
//! made for itself, and whose fingerprint travels in a QR code the phone
//! scans - so the phone trusts that one certificate and nobody's else,
//! which is what a certificate authority would otherwise be for. The QR
//! also carries the server's token. It is shown on this machine's screen
//! and read by a camera a hand's length away, which is the proof that the
//! person pairing is the person at the machine.
//!
//! Behind the handshake a connection does one of two things, by its first
//! line:
//!
//! - anything else is forwarded, byte for byte, to the JSON-RPC door, so
//!   the whole API is reachable over TLS and its own token check, its own
//!   limits and its own events apply unchanged;
//! - `{"method":"push", ...}` takes a project from the phone: a header
//!   line naming the files and their sizes, then the files' bytes, back to
//!   back. They land in a new folder under the inbox and nowhere else.

use std::fmt::Write as _;
use std::io::{self, Read, Write};
use std::net::{IpAddr, Shutdown, SocketAddr, TcpListener, TcpStream, UdpSocket};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use rustls::{ServerConfig, ServerConnection, StreamOwned};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::throttle::{Peer, Throttle};
use crate::token;

/// The longest header line a connection may open with.
const MAX_HEADER: usize = 256 * 1024;
/// How long a connection has to say what it wants.
const FIRST_LINE_PATIENCE: Duration = Duration::from_secs(5);
/// How long a push may go without a byte arriving.
const IDLE_PATIENCE: Duration = Duration::from_secs(30);
/// Most files in one push.
const MAX_FILES: usize = 5000;
/// Largest total a push may bring, in bytes.
const MAX_TOTAL: u64 = 16 * 1024 * 1024 * 1024;
/// Callers at once.
const MAX_CONNECTIONS: usize = 16;
/// How often a forwarding loop looks at both sides.
const TICK: Duration = Duration::from_millis(25);

/// The certificate this machine presents, and the fingerprint a phone pins.
pub struct Identity {
    cert: Vec<u8>,
    key: Vec<u8>,
    fingerprint: String,
}

impl Identity {
    /// Reads the identity kept in `dir`, or makes one and keeps it, so a
    /// phone paired once recognises the machine next time.
    pub fn load_or_create(dir: &Path) -> Result<Identity, String> {
        let cert_path = dir.join("pair-cert.der");
        let key_path = dir.join("pair-key.der");
        if let (Ok(cert), Ok(key)) = (std::fs::read(&cert_path), std::fs::read(&key_path)) {
            return Ok(Identity::of(cert, key));
        }
        let made = rcgen::generate_simple_self_signed(vec!["concat.local".to_owned()])
            .map_err(|error| format!("could not make a certificate: {error}"))?;
        let cert = made.cert.der().to_vec();
        let key = made.key_pair.serialize_der();
        std::fs::create_dir_all(dir)
            .map_err(|error| format!("could not create {}: {error}", dir.display()))?;
        std::fs::write(&cert_path, &cert)
            .map_err(|error| format!("could not write {}: {error}", cert_path.display()))?;
        std::fs::write(&key_path, &key)
            .map_err(|error| format!("could not write {}: {error}", key_path.display()))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&key_path, std::fs::Permissions::from_mode(0o600));
        }
        Ok(Identity::of(cert, key))
    }

    fn of(cert: Vec<u8>, key: Vec<u8>) -> Identity {
        let fingerprint = hex(&Sha256::digest(&cert));
        Identity {
            cert,
            key,
            fingerprint,
        }
    }

    /// SHA-256 of the certificate, as 64 lowercase hex digits.
    pub fn fingerprint(&self) -> &str {
        &self.fingerprint
    }
}

fn hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let _ = write!(out, "{byte:02x}");
    }
    out
}

/// What the QR code says: where to connect, what to pin, what to present.
pub fn pairing_url(host: IpAddr, port: u16, token: &str, fingerprint: &str) -> String {
    format!("concat://pair?h={host}&p={port}&t={token}&f={fingerprint}")
}

/// The address other machines on this network reach this one by: the
/// interface the system would send out of. Nothing is sent to find it.
pub fn lan_address() -> Option<IpAddr> {
    let socket = UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("192.0.2.1:9").ok()?;
    let ip = socket.local_addr().ok()?.ip();
    (!ip.is_unspecified() && !ip.is_loopback()).then_some(ip)
}

/// What a running pairing door needs to know.
pub struct Config {
    /// Where to listen for phones.
    pub listen: SocketAddr,
    /// The JSON-RPC door to forward to.
    pub forward_to: SocketAddr,
    /// The token that both doors take.
    pub token: String,
    /// Where pushed projects land.
    pub inbox: PathBuf,
}

/// The pairing door, running.
pub struct Pairing {
    addr: SocketAddr,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl Pairing {
    /// Binds `config.listen` and starts taking phones.
    pub fn start(identity: &Identity, config: Config) -> Result<Pairing, String> {
        let tls = tls_config(identity)?;
        let listener = TcpListener::bind(config.listen)
            .map_err(|error| format!("could not listen on {}: {error}", config.listen))?;
        let addr = listener
            .local_addr()
            .map_err(|error| format!("could not listen on {}: {error}", config.listen))?;
        let stop = Arc::new(AtomicBool::new(false));
        let context = Arc::new(Context {
            tls,
            forward_to: config.forward_to,
            token: config.token,
            inbox: config.inbox,
            throttle: Throttle::default(),
            seats: AtomicUsize::new(0),
            stop: Arc::clone(&stop),
        });
        let thread = std::thread::Builder::new()
            .name("concat pairing".to_owned())
            .spawn(move || accept_loop(&listener, &context))
            .map_err(|error| format!("could not start the pairing door: {error}"))?;
        Ok(Pairing {
            addr,
            stop,
            thread: Some(thread),
        })
    }

    /// The address bound, port resolved.
    pub fn addr(&self) -> SocketAddr {
        self.addr
    }
}

impl Drop for Pairing {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        let wake = if self.addr.ip().is_unspecified() {
            SocketAddr::new(std::net::Ipv4Addr::LOCALHOST.into(), self.addr.port())
        } else {
            self.addr
        };
        let _ = TcpStream::connect_timeout(&wake, Duration::from_secs(2));
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

struct Context {
    tls: Arc<ServerConfig>,
    forward_to: SocketAddr,
    token: String,
    inbox: PathBuf,
    throttle: Throttle,
    seats: AtomicUsize,
    stop: Arc<AtomicBool>,
}

fn tls_config(identity: &Identity) -> Result<Arc<ServerConfig>, String> {
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let config = ServerConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .map_err(|error| format!("TLS: {error}"))?
        .with_no_client_auth()
        .with_single_cert(
            vec![CertificateDer::from(identity.cert.clone())],
            PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(identity.key.clone())),
        )
        .map_err(|error| format!("TLS: {error}"))?;
    Ok(Arc::new(config))
}

fn accept_loop(listener: &TcpListener, context: &Arc<Context>) {
    for stream in listener.incoming() {
        if context.stop.load(Ordering::SeqCst) {
            return;
        }
        let Ok(stream) = stream else { continue };
        if context.seats.fetch_add(1, Ordering::SeqCst) >= MAX_CONNECTIONS {
            context.seats.fetch_sub(1, Ordering::SeqCst);
            continue;
        }
        let context = Arc::clone(context);
        let spawned = std::thread::Builder::new()
            .name("concat pairing caller".to_owned())
            .spawn({
                let context = Arc::clone(&context);
                move || {
                    let _ = connection(stream, &context);
                    context.seats.fetch_sub(1, Ordering::SeqCst);
                }
            });
        if spawned.is_err() {
            context.seats.fetch_sub(1, Ordering::SeqCst);
        }
    }
}

type Tls = StreamOwned<ServerConnection, TcpStream>;

fn connection(stream: TcpStream, context: &Context) -> io::Result<()> {
    let peer = stream
        .peer_addr()
        .map(|address| Peer::Ip(address.ip()))
        .unwrap_or(Peer::Local);
    stream.set_nodelay(true)?;
    stream.set_read_timeout(Some(TICK))?;
    stream.set_write_timeout(Some(Duration::from_secs(10)))?;
    let session = ServerConnection::new(Arc::clone(&context.tls)).map_err(io::Error::other)?;
    let mut tls: Tls = StreamOwned::new(session, stream);

    let line = read_line(&mut tls, FIRST_LINE_PATIENCE)?;
    let is_push = serde_json::from_slice::<Value>(&line)
        .ok()
        .is_some_and(|value| value.get("method").and_then(Value::as_str) == Some("push"));
    if is_push {
        let reply = match push(&mut tls, &line, context, peer) {
            Ok(done) => done,
            Err(why) => json!({ "ok": false, "error": why }),
        };
        let mut text = reply.to_string();
        text.push('\n');
        tls.write_all(text.as_bytes())?;
        tls.flush()?;
        tls.conn.send_close_notify();
        let _ = tls.flush();
        let _ = tls.sock.shutdown(Shutdown::Both);
        return Ok(());
    }
    forward(&mut tls, &line, context)
}

/// One line, without its newline, read a byte at a time so nothing after
/// it is taken.
fn read_line(tls: &mut Tls, patience: Duration) -> io::Result<Vec<u8>> {
    let deadline = Instant::now() + patience;
    let mut line = Vec::new();
    let mut byte = [0_u8; 1];
    loop {
        match tls.read(&mut byte) {
            Ok(0) => return Err(io::ErrorKind::UnexpectedEof.into()),
            Ok(_) if byte[0] == b'\n' => return Ok(line),
            Ok(_) => {
                line.push(byte[0]);
                if line.len() > MAX_HEADER {
                    return Err(io::Error::other("the first line is too long"));
                }
            }
            Err(error) if is_idle(&error) => {}
            Err(error) => return Err(error),
        }
        if Instant::now() > deadline {
            return Err(io::ErrorKind::TimedOut.into());
        }
    }
}

fn is_idle(error: &io::Error) -> bool {
    matches!(
        error.kind(),
        io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut | io::ErrorKind::Interrupted
    )
}

/// Everything after the first line, both ways, until either side ends.
fn forward(tls: &mut Tls, first: &[u8], context: &Context) -> io::Result<()> {
    let mut inner = TcpStream::connect_timeout(&context.forward_to, Duration::from_secs(3))?;
    inner.set_nodelay(true)?;
    inner.set_read_timeout(Some(TICK))?;
    inner.set_write_timeout(Some(Duration::from_secs(10)))?;
    inner.write_all(first)?;
    inner.write_all(b"\n")?;
    let mut buffer = vec![0_u8; 64 * 1024];
    loop {
        if context.stop.load(Ordering::SeqCst) {
            return Ok(());
        }
        match tls.read(&mut buffer) {
            Ok(0) => return Ok(()),
            Ok(count) => inner.write_all(&buffer[..count])?,
            Err(error) if is_idle(&error) => {}
            Err(error) => return Err(error),
        }
        match inner.read(&mut buffer) {
            Ok(0) => return Ok(()),
            Ok(count) => {
                tls.write_all(&buffer[..count])?;
                tls.flush()?;
            }
            Err(error) if is_idle(&error) => {}
            Err(error) => return Err(error),
        }
    }
}

/// A project pushed from a phone: checks the token, then writes each file
/// under a new folder in the inbox.
fn push(tls: &mut Tls, line: &[u8], context: &Context, peer: Peer) -> Result<Value, String> {
    if let Some(wait) = context.throttle.wait(peer) {
        return Err(format!("wrong tokens lately; wait {}s", wait.as_secs() + 1));
    }
    let request: Value = serde_json::from_slice(line).map_err(|_| "unreadable header")?;
    let params = request.get("params").unwrap_or(&request);
    let presented = params.get("token").and_then(Value::as_str).unwrap_or("");
    if !token::matches(presented.as_bytes(), context.token.as_bytes()) {
        context.throttle.failed(peer);
        return Err("wrong token".to_owned());
    }
    context.throttle.succeeded(peer);

    let name = params
        .get("project")
        .and_then(Value::as_str)
        .map(folder_name)
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "project".to_owned());
    let files = params
        .get("files")
        .and_then(Value::as_array)
        .ok_or("the header names no files")?;
    if files.is_empty() || files.len() > MAX_FILES {
        return Err(format!("between 1 and {MAX_FILES} files"));
    }
    let mut plan = Vec::with_capacity(files.len());
    let mut total = 0_u64;
    for file in files {
        let path = file
            .get("path")
            .and_then(Value::as_str)
            .and_then(safe_path)
            .ok_or("a file has an unsafe path")?;
        let size = file
            .get("size")
            .and_then(Value::as_u64)
            .ok_or("a file has no size")?;
        total = total.saturating_add(size);
        plan.push((path, size));
    }
    if total > MAX_TOTAL {
        return Err("the project is larger than this machine takes".to_owned());
    }

    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_secs());
    let folder = context.inbox.join(format!("{name}-{stamp}"));
    std::fs::create_dir_all(&folder).map_err(|error| format!("inbox: {error}"))?;
    for (path, size) in &plan {
        let target = folder.join(path);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent).map_err(|error| format!("inbox: {error}"))?;
        }
        let mut out = std::fs::File::create(&target).map_err(|error| format!("inbox: {error}"))?;
        receive(tls, &mut out, *size).map_err(|error| format!("transfer: {error}"))?;
    }
    Ok(json!({
        "ok": true,
        "path": folder.display().to_string(),
        "files": plan.len(),
        "bytes": total,
    }))
}

/// Exactly `size` bytes from the stream into `out`.
fn receive(tls: &mut Tls, out: &mut impl Write, size: u64) -> io::Result<()> {
    let mut left = size;
    let mut buffer = vec![0_u8; 256 * 1024];
    let mut last = Instant::now();
    while left > 0 {
        let want = buffer.len().min(usize::try_from(left).unwrap_or(usize::MAX));
        match tls.read(&mut buffer[..want]) {
            Ok(0) => return Err(io::ErrorKind::UnexpectedEof.into()),
            Ok(count) => {
                out.write_all(&buffer[..count])?;
                left -= count as u64;
                last = Instant::now();
            }
            Err(error) if is_idle(&error) => {
                if last.elapsed() > IDLE_PATIENCE {
                    return Err(io::ErrorKind::TimedOut.into());
                }
            }
            Err(error) => return Err(error),
        }
    }
    out.flush()
}

/// A project name fit to be a folder name: letters, digits and a few
/// marks, at most 48 of them.
fn folder_name(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || matches!(c, '-' | '_') {
                c
            } else {
                '_'
            }
        })
        .take(48)
        .collect::<String>()
        .trim_matches('_')
        .to_owned()
}

/// A relative path a phone may name: plain components only, no way up or
/// out, nothing Windows would read as a drive or a device.
fn safe_path(text: &str) -> Option<PathBuf> {
    if text.is_empty() || text.len() > 400 || text.contains(['\\', ':', '\0']) {
        return None;
    }
    let mut clean = PathBuf::new();
    for part in text.split('/') {
        if part.is_empty()
            || part == "."
            || part == ".."
            || part.chars().any(|c| c.is_control() || "<>\"|?*".contains(c))
            || part.ends_with('.')
            || part.ends_with(' ')
        {
            return None;
        }
        clean.push(part);
    }
    (!clean.as_os_str().is_empty()).then_some(clean)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_that_climb_or_name_a_drive_are_refused() {
        for bad in [
            "../x", "a/../../x", "/etc/passwd", "C:/x", "a\\b", "a/./", "", "con:", "a/b.", "a/<b>",
        ] {
            assert!(safe_path(bad).is_none(), "{bad:?}");
        }
        assert_eq!(
            safe_path("media/clip 1.mp4"),
            Some(PathBuf::from("media/clip 1.mp4"))
        );
        assert_eq!(safe_path("concat.json"), Some(PathBuf::from("concat.json")));
    }

    #[test]
    fn a_project_name_becomes_a_plain_folder_name() {
        assert_eq!(folder_name("../../Proyek Saya!"), "Proyek_Saya");
        assert_eq!(folder_name("???"), "");
        assert!(folder_name(&"a".repeat(200)).len() <= 48);
    }

    #[test]
    fn the_pairing_url_carries_everything_a_phone_needs() {
        let url = pairing_url("192.168.1.20".parse().expect("ip"), 7421, "tok", "ab12");
        assert_eq!(url, "concat://pair?h=192.168.1.20&p=7421&t=tok&f=ab12");
    }

    #[test]
    fn an_identity_is_kept_and_recognised() {
        let dir = tempfile::tempdir().expect("scratch");
        let first = Identity::load_or_create(dir.path()).expect("made");
        let again = Identity::load_or_create(dir.path()).expect("read");
        assert_eq!(first.fingerprint(), again.fingerprint());
        assert_eq!(first.fingerprint().len(), 64);
    }
}
