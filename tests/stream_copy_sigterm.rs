use std::io::Read;
use std::os::fd::AsRawFd;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

struct RedisServer {
    child: Child,
    port: u16,
}

impl RedisServer {
    /// Starts a throwaway redis-server on a free port. Returns None when the binary is not
    /// available, so the test can be skipped in that environment.
    fn start() -> Option<Self> {
        let port = std::net::TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port();

        let child = Command::new("redis-server")
            .args(["--port", &port.to_string(), "--bind", "127.0.0.1"])
            .args(["--save", "", "--appendonly", "no"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .ok()?;

        let server = Self { child, port };
        for _ in 0..50 {
            if server.conn(0).is_ok() {
                return Some(server);
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        panic!("redis-server did not become ready");
    }

    fn conn(&self, db: u8) -> redis::RedisResult<redis::Connection> {
        let mut conn = redis::Client::open(format!("redis://127.0.0.1:{}/{db}", self.port))?
            .get_connection()?;
        redis::cmd("PING").query::<String>(&mut conn)?;
        Ok(conn)
    }
}

impl Drop for RedisServer {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[test]
fn stream_copy_exits_cleanly_on_sigterm() {
    let Some(redis) = RedisServer::start() else {
        eprintln!("redis-server not available, skipping");
        return;
    };
    let port = redis.port.to_string();

    // Run from an empty directory so no redito.toml/local_config.toml is picked up.
    let dir = std::env::temp_dir().join(format!("redito-sigterm-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();

    // Copy db 0 -> db 1 of the same server, waiting for entries like the deployed copier.
    let mut child = Command::new(env!("CARGO_BIN_EXE_redito"))
        .args(["stream-copy", "--stream", "test-stream"])
        .current_dir(&dir)
        .env("REDITO_REDIS__HOST", "127.0.0.1")
        .env("REDITO_REDIS__PORT", &port)
        .env("REDITO_REDIS__DB", "0")
        .env("REDITO_COMMAND__TARGET__HOST", "127.0.0.1")
        .env("REDITO_COMMAND__TARGET__PORT", &port)
        .env("REDITO_COMMAND__TARGET__DB", "1")
        .env("REDITO_COMMAND__RETRY_WHEN_EMPTY", "true")
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    // It tails from `$`, so keep adding entries until one shows up on the target: from then
    // on the copy loop is running.
    let mut source = redis.conn(0).unwrap();
    let mut target = redis.conn(1).unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        redis::cmd("XADD")
            .arg("test-stream")
            .arg("*")
            .arg("k")
            .arg("v")
            .query::<String>(&mut source)
            .unwrap();
        let copied: usize = redis::cmd("XLEN")
            .arg("test-stream")
            .query(&mut target)
            .unwrap();
        if copied > 0 {
            break;
        }
        if Instant::now() > deadline {
            let _ = child.kill();
            panic!("stream-copy copied nothing within 10s");
        }
        std::thread::sleep(Duration::from_millis(100));
    }

    // Nothing more is added, so stream-copy is now blocked in XREAD (block_ms 5000).
    std::thread::sleep(Duration::from_millis(200));
    unsafe { libc::kill(child.id() as libc::pid_t, libc::SIGTERM) };

    let deadline = Instant::now() + Duration::from_secs(3);
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if Instant::now() > deadline {
            let _ = child.kill();
            panic!("stream-copy still running 3s after SIGTERM");
        }
        std::thread::sleep(Duration::from_millis(50));
    };
    let stderr = std::io::read_to_string(child.stderr.take().unwrap()).unwrap();
    let _ = std::fs::remove_dir_all(&dir);

    assert!(
        status.success(),
        "expected a clean exit on SIGTERM, got {status}; stderr: {stderr}"
    );
    assert!(
        stderr.contains("Received SIGTERM"),
        "expected a shutdown message on stderr; stderr: {stderr}"
    );
}

#[test]
fn stream_tail_exits_cleanly_on_sigterm() {
    let Some(redis) = RedisServer::start() else {
        eprintln!("redis-server not available, skipping");
        return;
    };
    let port = redis.port.to_string();

    // Run from an empty directory so no redito.toml/local_config.toml is picked up.
    let dir = std::env::temp_dir().join(format!("redito-tail-sigterm-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();

    let mut child = Command::new(env!("CARGO_BIN_EXE_redito"))
        .args(["stream-tail", "--stream", "test-stream"])
        .current_dir(&dir)
        .env("REDITO_REDIS__HOST", "127.0.0.1")
        .env("REDITO_REDIS__PORT", &port)
        .env("REDITO_REDIS__DB", "0")
        .env("REDITO_COMMAND__RETRY_WHEN_EMPTY", "true")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    let mut stdout = child.stdout.take().expect("child should have stdout");
    // Non-blocking, so waiting for output never stops us from adding more entries.
    unsafe {
        let fd = stdout.as_raw_fd();
        let flags = libc::fcntl(fd, libc::F_GETFL);
        assert!(flags >= 0, "F_GETFL failed");
        assert_eq!(
            libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK),
            0,
            "F_SETFL failed"
        );
    }

    // It tails from `$`, so keep adding entries until one shows up on the target: from then
    // on the copy loop is running.
    let mut source = redis.conn(0).unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut output = Vec::new();
    loop {
        redis::cmd("XADD")
            .arg("test-stream")
            .arg("*")
            .arg("k")
            .arg("v")
            .query::<String>(&mut source)
            .unwrap();

        let mut buf = [0; 64];
        match stdout.read(&mut buf) {
            Ok(0) => panic!("stream-tail closed stdout before printing anything"),
            Ok(n) => output.extend_from_slice(&buf[..n]),
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
            Err(e) => panic!("reading stream-tail stdout: {e}"),
        }
        if output.contains(&b'\n') {
            break;
        }
        if Instant::now() > deadline {
            let _ = child.kill();
            panic!("stream-copy copied nothing within 10s");
        }
        std::thread::sleep(Duration::from_millis(100));
    }

    // Nothing more is added, so stream-copy is now blocked in XREAD (block_ms 5000).
    std::thread::sleep(Duration::from_millis(200));
    unsafe { libc::kill(child.id() as libc::pid_t, libc::SIGTERM) };

    let deadline = Instant::now() + Duration::from_secs(3);
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if Instant::now() > deadline {
            let _ = child.kill();
            panic!("stream-copy still running 3s after SIGTERM");
        }
        std::thread::sleep(Duration::from_millis(50));
    };
    let stderr = std::io::read_to_string(child.stderr.take().unwrap()).unwrap();
    let _ = std::fs::remove_dir_all(&dir);

    assert!(
        status.success(),
        "expected a clean exit on SIGTERM, got {status}; stderr: {stderr}"
    );
    assert!(
        stderr.contains("Received SIGTERM"),
        "expected a shutdown message on stderr; stderr: {stderr}"
    );
}
