use super::{Cause, Completed, Failure, Limits, Observer, State, Streams};
use std::process::{Command, ExitStatus, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::process::Child;

pub(super) async fn execute<I, O, E>(
    command: Command,
    mut streams: Streams<I, O, E>,
    limits: Limits,
    end: super::observe::End,
) -> Result<Completed<Streams<I, O, E>>, Failure<Streams<I, O, E>>>
where
    I: AsyncRead + Unpin,
    O: AsyncWrite + Unpin,
    E: AsyncWrite + Unpin,
{
    let observer = end.0.clone();
    let _end = end;
    if observer.cancel.cancelled() {
        observer.set(State::Settled);
        return Err(failure(Cause::Cancelled, streams));
    }
    let mut command = tokio::process::Command::from(command);
    command
        .stdin(disposition(streams.input.is_some()))
        .stdout(disposition(streams.stdout.is_some()))
        .stderr(disposition(streams.stderr.is_some()))
        .kill_on_drop(true);
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(error) => {
            observer.set(State::Settled);
            return Err(failure(Cause::Io(error), streams));
        }
    };
    let result = tokio::select! {
        result = exchange(&mut child, &mut streams, limits.output) => result,
        _ = tokio::time::sleep(limits.time) => Err(Cause::Time),
        _ = observer.cancel.wait() => Err(Cause::Cancelled),
    };
    match result {
        Ok(status) => {
            observer.set(State::Settled);
            Ok(Completed { status, streams })
        }
        Err(cause) => Err(cleanup(child, failure(cause, streams), observer).await),
    }
}

async fn cleanup<T>(mut child: Child, mut failed: Failure<T>, observer: Observer) -> Failure<T> {
    observer.set(State::Stopping);
    match stop(&mut child).await {
        Ok(status) => {
            failed.status = Some(status);
            observer.set(State::Settled);
        }
        Err(error) => {
            failed.cleanup = Some(error);
            failed.child = Some(child);
            observer.set(State::Uncertain);
        }
    }
    failed
}

fn failure<T>(cause: Cause, streams: T) -> Failure<T> {
    Failure {
        cause,
        streams,
        status: None,
        cleanup: None,
        child: None,
    }
}

async fn stop(child: &mut Child) -> Result<ExitStatus, std::io::Error> {
    if let Some(status) = child.try_wait()? {
        return Ok(status);
    }
    child.start_kill()?;
    child.wait().await
}

async fn exchange<I, O, E>(
    child: &mut Child,
    streams: &mut Streams<I, O, E>,
    limit: u64,
) -> Result<ExitStatus, Cause>
where
    I: AsyncRead + Unpin,
    O: AsyncWrite + Unpin,
    E: AsyncWrite + Unpin,
{
    let input = child.stdin.take();
    let output = child.stdout.take();
    let errors = child.stderr.take();
    let bytes = Arc::new(AtomicU64::new(0));
    let send = supply(input, streams.input.as_mut());
    let wait = async { child.wait().await.map_err(Cause::Io) };
    let drain = async {
        tokio::try_join!(
            send,
            transfer(output, streams.stdout.as_mut(), bytes.clone(), limit),
            transfer(errors, streams.stderr.as_mut(), bytes, limit)
        )?;
        Ok(())
    };
    let (status, ()) = tokio::try_join!(wait, drain)?;
    Ok(status)
}

fn disposition(pipe: bool) -> Stdio {
    if pipe { Stdio::piped() } else { Stdio::null() }
}

async fn supply<I: AsyncRead + Unpin>(
    pipe: Option<tokio::process::ChildStdin>,
    source: Option<&mut I>,
) -> Result<(), Cause> {
    match (pipe, source) {
        (Some(mut pipe), Some(source)) => {
            tokio::io::copy(source, &mut pipe)
                .await
                .map_err(Cause::Io)?;
            pipe.shutdown().await.map_err(Cause::Io)
        }
        (None, None) => Ok(()),
        _ => Err(mismatch()),
    }
}

fn mismatch() -> Cause {
    Cause::Io(std::io::Error::other("stream disposition mismatch"))
}

async fn transfer(
    source: Option<impl AsyncRead + Unpin>,
    sink: Option<&mut (impl AsyncWrite + Unpin)>,
    bytes: Arc<AtomicU64>,
    limit: u64,
) -> Result<(), Cause> {
    match (source, sink) {
        (Some(source), Some(sink)) => drain(source, sink, bytes, limit).await,
        (None, None) => Ok(()),
        _ => Err(mismatch()),
    }
}

async fn drain(
    mut source: impl AsyncRead + Unpin,
    sink: &mut (impl AsyncWrite + Unpin),
    bytes: Arc<AtomicU64>,
    limit: u64,
) -> Result<(), Cause> {
    let mut buffer = [0; 8192];
    loop {
        let count = source.read(&mut buffer).await.map_err(Cause::Io)?;
        if count == 0 {
            break;
        }
        bytes
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |used| {
                used.checked_add(count as u64).filter(|sum| *sum <= limit)
            })
            .map_err(|_| Cause::Output)?;
        sink.write_all(&buffer[..count]).await.map_err(Cause::Io)?;
    }
    sink.flush().await.map_err(Cause::Io)
}
