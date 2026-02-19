use crate::error::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SseEvent {
    pub event: Option<String>,
    pub data: String,
}

#[derive(Debug, Default)]
pub struct SseParser {
    buf: Vec<u8>,
}

impl SseParser {
    pub fn new() -> Self {
        Self { buf: Vec::new() }
    }

    pub fn push(&mut self, chunk: &[u8]) -> Result<Vec<SseEvent>, Error> {
        self.buf.extend_from_slice(chunk);
        let mut out = Vec::new();
        while let Some((frame_end, consume_end)) = find_frame_boundary(&self.buf) {
            let frame = self.buf[..frame_end].to_vec();
            self.buf.drain(..consume_end);
            if let Some(event) = parse_frame(&frame)? {
                out.push(event);
            }
        }
        Ok(out)
    }

    pub fn finish(&mut self) -> Result<Option<SseEvent>, Error> {
        if self.buf.is_empty() {
            return Ok(None);
        }
        let frame = std::mem::take(&mut self.buf);
        parse_frame(&frame)
    }
}

fn find_frame_boundary(buf: &[u8]) -> Option<(usize, usize)> {
    let mut i = 0;
    while i + 1 < buf.len() {
        if buf[i] == b'\n' && buf[i + 1] == b'\n' {
            return Some((i, i + 2));
        }
        if i + 3 < buf.len()
            && buf[i] == b'\r'
            && buf[i + 1] == b'\n'
            && buf[i + 2] == b'\r'
            && buf[i + 3] == b'\n'
        {
            return Some((i, i + 4));
        }
        i += 1;
    }
    None
}

fn parse_frame(frame: &[u8]) -> Result<Option<SseEvent>, Error> {
    let text = std::str::from_utf8(frame).map_err(|e| Error::InvalidSse(e.to_string()))?;

    let mut event: Option<String> = None;
    let mut data_lines: Vec<&str> = Vec::new();

    for mut line in text.split('\n') {
        line = line.trim_end_matches('\r');
        if line.is_empty() || line.starts_with(':') {
            continue;
        }

        let (field, value) = match line.split_once(':') {
            Some((f, v)) => (f, v.strip_prefix(' ').unwrap_or(v)),
            None => (line, ""),
        };

        match field {
            "event" => {
                let v = value.trim();
                event = if v.is_empty() {
                    None
                } else {
                    Some(v.to_string())
                };
            }
            "data" => data_lines.push(value),
            _ => {}
        }
    }

    let data = data_lines.join("\n");
    if event.is_none() && data.is_empty() {
        return Ok(None);
    }
    Ok(Some(SseEvent { event, data }))
}
