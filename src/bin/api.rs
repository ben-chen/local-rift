//! One JSON request per input line; one JSON response per output line.
use local_rift::api::{MAX_REQUEST_BYTES, Session};
use serde_json::json;
use std::io::{self, BufRead, Write};

/// Consume even an oversized record without accumulating unbounded input.
fn record(reader: &mut impl BufRead) -> io::Result<Option<Result<String, &'static str>>> {
    let mut bytes = Vec::new();
    let mut oversized = false;
    let mut saw_input = false;
    loop {
        let available = reader.fill_buf()?;
        if available.is_empty() {
            break;
        }
        saw_input = true;
        let newline = available.iter().position(|&byte| byte == b'\n');
        let count = newline.map_or(available.len(), |index| index + 1);
        if !oversized {
            if bytes.len() + count > MAX_REQUEST_BYTES {
                oversized = true;
                bytes.clear();
            } else {
                bytes.extend_from_slice(&available[..count]);
            }
        }
        reader.consume(count);
        if newline.is_some() {
            break;
        }
    }
    if !saw_input {
        return Ok(None);
    }
    if oversized {
        return Ok(Some(Err("request_too_large")));
    }
    Ok(Some(String::from_utf8(bytes).map_err(|_| "invalid_utf8")))
}

fn main() -> io::Result<()> {
    if std::env::args()
        .skip(1)
        .any(|arg| arg == "--help" || arg == "-h")
    {
        eprintln!(
            "rift-api: offline JSONL game server on stdin/stdout. See docs/API.md. Send {{\"command\":\"info\"}} to begin."
        );
        return Ok(());
    }
    let mut session = Session::default();
    let mut input = io::stdin().lock();
    let mut output = io::BufWriter::new(io::stdout().lock());
    while let Some(line) = record(&mut input)? {
        let response = match line {
            Ok(line) => session.process_line(&line),
            Err(code) => {
                json!({"ok":false,"revision":session.revision(),"error":{"code":code,"message":"Request must be UTF-8 JSON and at most 1 MiB."}})
            }
        };
        serde_json::to_writer(&mut output, &response)?;
        output.write_all(b"\n")?;
        output.flush()?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn oversized_and_invalid_utf8_records_do_not_consume_the_next_request() {
        let mut bytes = vec![b'x'; MAX_REQUEST_BYTES + 2];
        bytes.extend_from_slice(b"\n{\"command\":\"info\"}\n\xff\nlast");
        let mut input = io::Cursor::new(bytes);
        assert_eq!(record(&mut input).unwrap(), Some(Err("request_too_large")));
        assert_eq!(
            record(&mut input).unwrap().unwrap().unwrap(),
            "{\"command\":\"info\"}\n"
        );
        assert_eq!(record(&mut input).unwrap(), Some(Err("invalid_utf8")));
        assert_eq!(record(&mut input).unwrap(), Some(Ok("last".into())));
        assert!(record(&mut input).unwrap().is_none());
    }
}
