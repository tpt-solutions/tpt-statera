// SPDX-License-Identifier: MIT OR Apache-2.0
// Copyright (c) 2026 TPT Solutions

//! Single-writer service (todo 2B.05, cross-process serialization for off-chain deployments).
//!
//! Recommendation adopted (ARCHITECTURE AD-10): one thread owns the [`Ledger`] and is the only
//! writer. Every submission, from any thread or any process, is queued and executed one at a
//! time. The writer calls [`Ledger::submit`], so the per-transfer guarantees are unchanged.
//!
//! * In-process clients use [`WriterService`]. It is cheaply cloneable.
//! * Cross-process clients use [`serve_tcp`] on the server side and [`TcpClient`] on the other
//!   side. The wire protocol is one line per request and one line per reply. It is plain text and
//!   has no authentication. Bind to loopback or a private network only (see the scope notes).
//!
//! ## Scope
//!
//! * Serialization is provided by the single writer. The writer is a single point of failure and
//!   a throughput bottleneck. That is the trade-off of this design.
//! * The protocol has no authentication, no TLS and no authorization. Any client that can reach the
//!   port can move funds from any account. Deployments must add access control at the transport
//!   layer. This crate does not.
//! * A malformed request gets an `E` reply and does not change state.

use std::io::{self, BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream, ToSocketAddrs};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self, JoinHandle};

use tpt_statera_core::ValidationOutcome;

use crate::{AccountId, Ledger, LedgerError, Submission};

/// Errors from the service layer.
#[derive(Debug)]
pub enum ServiceError {
    /// The writer thread has stopped, so the request could not be delivered.
    WriterGone,
    /// The ledger rejected the request (unknown account, self-transfer, poisoned lock).
    Ledger(LedgerError),
    /// A transport or protocol error on the TCP path.
    Io(io::Error),
    /// The server replied with something this client does not understand.
    Protocol(String),
}

impl From<io::Error> for ServiceError {
    fn from(e: io::Error) -> Self {
        ServiceError::Io(e)
    }
}

/// A reply from a submission. Mirrors [`Submission`] with the fee exposed on the wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reply {
    /// Committed. Carries the whole-unit fee charged.
    Committed {
        /// Fee charged to the sender, in whole units.
        fee: u64,
    },
    /// Denied by validation. No state changed.
    Denied,
    /// Snapshot went stale. No state changed. The caller may resubmit.
    Stale,
}

impl From<Submission> for Reply {
    fn from(s: Submission) -> Self {
        match s {
            Submission::Committed(ValidationOutcome::Allow { rounded_fee, .. }) => {
                Reply::Committed { fee: rounded_fee }
            }
            // The ledger never commits a Deny. Map it defensively to Denied.
            Submission::Committed(ValidationOutcome::Deny) | Submission::Denied => Reply::Denied,
            Submission::Stale => Reply::Stale,
        }
    }
}

struct Request {
    sender: AccountId,
    recipient: AccountId,
    amount: u64,
    reply: Sender<Result<Reply, LedgerError>>,
}

/// Cloneable in-process handle to the single writer.
#[derive(Clone)]
pub struct WriterService {
    tx: Sender<Request>,
}

impl WriterService {
    /// Start the writer thread. It owns `ledger` until every handle is dropped.
    pub fn spawn(ledger: Ledger) -> (WriterService, JoinHandle<Ledger>) {
        let (tx, rx) = mpsc::channel::<Request>();
        let handle = thread::spawn(move || run_writer(ledger, rx));
        (WriterService { tx }, handle)
    }

    /// Submit one transfer and wait for the writer's reply.
    pub fn submit(
        &self,
        sender: AccountId,
        recipient: AccountId,
        amount: u64,
    ) -> Result<Reply, ServiceError> {
        let (reply_tx, reply_rx) = mpsc::channel();
        self.tx
            .send(Request {
                sender,
                recipient,
                amount,
                reply: reply_tx,
            })
            .map_err(|_| ServiceError::WriterGone)?;
        let result = reply_rx.recv().map_err(|_| ServiceError::WriterGone)?;
        result.map_err(ServiceError::Ledger)
    }
}

fn run_writer(ledger: Ledger, rx: Receiver<Request>) -> Ledger {
    // One request at a time. This loop is the serialization point.
    while let Ok(req) = rx.recv() {
        let result = ledger
            .submit(req.sender, req.recipient, req.amount)
            .map(Reply::from);
        // A dropped reply channel means the client went away. The transfer already happened
        // or was denied, and the ledger is consistent either way.
        let _ = req.reply.send(result);
    }
    ledger
}

/// Serve the line protocol on `listener`, forwarding every request to `service`.
///
/// Protocol (each request is one line, each reply is one line):
/// * `T <sender> <recipient> <amount>` → `C <fee>`, `D`, `S`, or `E <message>`
///
/// Each connection is handled on its own thread. All writes still go through the single writer.
/// The function returns only if `listener.accept()` fails.
pub fn serve_tcp(listener: TcpListener, service: WriterService) -> io::Result<()> {
    loop {
        let (stream, _) = listener.accept()?;
        let svc = service.clone();
        thread::spawn(move || {
            let _ = handle_connection(stream, svc);
        });
    }
}

fn handle_connection(stream: TcpStream, service: WriterService) -> io::Result<()> {
    let mut writer = stream.try_clone()?;
    let reader = BufReader::new(stream);
    for line in reader.lines() {
        let line = line?;
        let reply = match parse_request(&line) {
            Ok((s, r, a)) => match service.submit(s, r, a) {
                Ok(Reply::Committed { fee }) => format!("C {fee}"),
                Ok(Reply::Denied) => "D".to_string(),
                Ok(Reply::Stale) => "S".to_string(),
                Err(ServiceError::Ledger(e)) => format!("E {e:?}"),
                Err(e) => format!("E {e:?}"),
            },
            Err(msg) => format!("E {msg}"),
        };
        writeln!(writer, "{reply}")?;
    }
    Ok(())
}

fn parse_request(line: &str) -> Result<(AccountId, AccountId, u64), String> {
    let mut it = line.split_whitespace();
    if it.next() != Some("T") {
        return Err("expected `T <sender> <recipient> <amount>`".into());
    }
    let parse = |s: Option<&str>, what: &str| -> Result<u64, String> {
        s.ok_or_else(|| format!("missing {what}"))?
            .parse::<u64>()
            .map_err(|_| format!("{what} is not an unsigned integer"))
    };
    let sender = parse(it.next(), "sender")?;
    let recipient = parse(it.next(), "recipient")?;
    let amount = parse(it.next(), "amount")?;
    if it.next().is_some() {
        return Err("trailing input".into());
    }
    Ok((sender, recipient, amount))
}

/// Client for the line protocol. Holds one TCP connection; requests on it are sequential.
pub struct TcpClient {
    writer: TcpStream,
    reader: BufReader<TcpStream>,
}

impl TcpClient {
    /// Connect to a server started with [`serve_tcp`].
    pub fn connect<A: ToSocketAddrs>(addr: A) -> io::Result<Self> {
        let stream = TcpStream::connect(addr)?;
        let reader = BufReader::new(stream.try_clone()?);
        Ok(Self {
            writer: stream,
            reader,
        })
    }

    /// Submit a transfer and read the reply line.
    pub fn submit(
        &mut self,
        sender: AccountId,
        recipient: AccountId,
        amount: u64,
    ) -> Result<Reply, ServiceError> {
        writeln!(self.writer, "T {sender} {recipient} {amount}")?;
        let mut line = String::new();
        if self.reader.read_line(&mut line)? == 0 {
            return Err(ServiceError::Protocol("connection closed".into()));
        }
        let line = line.trim_end();
        match line.split_once(' ') {
            Some(("C", fee)) => fee
                .parse::<u64>()
                .map(|fee| Reply::Committed { fee })
                .map_err(|_| ServiceError::Protocol(format!("bad fee in `{line}`"))),
            None if line == "D" => Ok(Reply::Denied),
            None if line == "S" => Ok(Reply::Stale),
            Some(("E", msg)) => Err(ServiceError::Protocol(msg.to_string())),
            _ => Err(ServiceError::Protocol(format!(
                "unrecognized reply `{line}`"
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    #[test]
    fn parse_accepts_well_formed_and_rejects_the_rest() {
        assert_eq!(parse_request("T 1 2 300"), Ok((1, 2, 300)));
        assert!(parse_request("X 1 2 3").is_err());
        assert!(parse_request("T 1 2").is_err());
        assert!(parse_request("T 1 2 -3").is_err());
        assert!(parse_request("T 1 2 3 4").is_err());
        assert!(parse_request("").is_err());
    }

    #[test]
    fn writer_serializes_and_conserves_under_many_clients() {
        let ledger = Ledger::new(&[(1, 100_000), (2, 0), (3, 0)], 25);
        let total = ledger.total().unwrap();
        let (svc, handle) = WriterService::spawn(ledger);
        let mut threads = Vec::new();
        for t in 0..6u64 {
            let svc = svc.clone();
            threads.push(thread::spawn(move || {
                for i in 0..200u64 {
                    let to = if (t + i) % 2 == 0 { 2 } else { 3 };
                    let _ = svc.submit(1, to, 5 + (i % 7));
                }
            }));
        }
        for th in threads {
            th.join().unwrap();
        }
        drop(svc);
        let ledger = handle.join().unwrap();
        assert_eq!(ledger.total().unwrap(), total);
    }

    #[test]
    fn tcp_round_trip_commits_denies_and_rejects_garbage() {
        let ledger = Ledger::new(&[(1, 1_000), (2, 0)], 0);
        let (svc, _handle) = WriterService::spawn(ledger);
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let addr = listener.local_addr().unwrap();
        thread::spawn(move || {
            let _ = serve_tcp(listener, svc);
        });

        let mut client = TcpClient::connect(addr).unwrap();
        assert_eq!(
            client.submit(1, 2, 400).unwrap(),
            Reply::Committed { fee: 0 }
        );
        assert_eq!(client.submit(1, 2, 700).unwrap(), Reply::Denied);

        // Garbage gets an error reply, and the connection stays usable.
        writeln!(client.writer, "T nope 2 3").unwrap();
        let mut line = String::new();
        client.reader.read_line(&mut line).unwrap();
        assert!(line.starts_with("E "), "got {line:?}");
        assert_eq!(
            client.submit(1, 2, 100).unwrap(),
            Reply::Committed { fee: 0 }
        );
    }
}
