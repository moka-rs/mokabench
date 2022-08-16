pub trait TraceParser<T> {
    fn parse(&mut self, line: &str, line_number: usize) -> anyhow::Result<Option<T>>;
}

#[derive(Debug)]
pub struct TraceEntry {
    range: std::ops::Range<usize>,
    line_number: usize,
}

impl TraceEntry {
    pub fn range(&self) -> std::ops::Range<usize> {
        self.range.clone()
    }

    pub fn line_number(&self) -> usize {
        self.line_number
    }
}

// Arc traces contains a 2+ numbers per line, the first two being start and len, meaning a range `start..start+len`
// LIRS/LIRS2 traces contains a single key per line
pub struct GenericTraceParser;

impl TraceParser<TraceEntry> for GenericTraceParser {
    fn parse(&mut self, line: &str, line_number: usize) -> anyhow::Result<Option<TraceEntry>> {
        if line == "*" {
            // LIRS/LIRS2 traces contains `*` lines which are NOOPs
            return Ok(None);
        }
        let mut tokens = line.split(' ');
        let start = if let Some(token) = tokens.next() {
            token.parse::<usize>()?
        } else {
            anyhow::bail!("Expected at least one integer in the line: {}", line);
        };
        let len = if let Some(token) = tokens.next() {
            token.parse::<usize>()?
        } else {
            // single integer per line format
            1
        };

        Ok(Some(TraceEntry {
            range: start..(start + len),
            line_number,
        }))
    }
}
